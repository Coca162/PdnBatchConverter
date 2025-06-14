use std::{
    collections::{BTreeMap, HashMap},
    convert::identity,
    fmt::Write,
    fs::{create_dir_all, read_dir},
    iter, mem,
    num::NonZeroUsize,
    ops::Not,
    path::{Path, PathBuf},
    sync::{
        Arc,
        atomic::{AtomicBool, Ordering},
    },
};

use iced::{
    Alignment, Border, Color, Element, Font, Length, Padding, Size, Subscription, Task, Theme,
    alignment::{Horizontal, Vertical},
    clipboard, font,
    futures::{StreamExt, stream},
    mouse::Interaction,
    task::sipper,
    widget::{
        Button, Column, Container, MouseArea, Row, Scrollable, Stack, Svg, Text, button, checkbox,
        container::{self, background},
        row, scrollable, svg,
        text::{self, Rich, Span},
        tooltip,
    },
    window::{self, Position, Settings},
};
use iced::{
    Background,
    widget::{
        pick_list,
        scrollable::{Direction, Rail, Scrollbar, Scroller},
    },
};
use rfd::FileHandle;

use dialog::Dialog;
use folder_error::{FolderSearchError, SelectFolderErrors};
use pdn_conv::{DEFAULT_LOCATION, State, StateInitError, VERSION};

mod dialog;
mod folder_error;
mod icons;

const ERROR_WINDOW_SIZE: Size = Size {
    width: 450.,
    height: 250.,
};

pub const MAX_FILE_SEARCH: usize = 2_000_000;

pub fn main() -> iced::Result {
    iced::daemon(
        || {
            let state = State::init();

            match state {
                Ok(hoster_state) => OraConverterGui::main_window(hoster_state),
                Err(error) => {
                    let (id, t) = window::open(Settings {
                        position: Position::Centered,
                        size: ERROR_WINDOW_SIZE,
                        resizable: false,
                        ..Default::default()
                    });
                    (
                        OraConverterGui::SetupIssue(SetupState {
                            id,
                            error: eyre::Report::new(error),
                        }),
                        t.discard(),
                    )
                }
            }
        },
        OraConverterGui::update,
        OraConverterGui::view,
    )
    .title(|s: &OraConverterGui, w_id| {
        let title = match s {
            OraConverterGui::SetupIssue(_) => "Setup",
            OraConverterGui::MainWindow(MainState { id, .. }) if *id == w_id => {
                "PDN Batch Converter"
            }
            OraConverterGui::MainWindow(MainState { dialog_windows, .. }) => {
                match dialog_windows.get(&w_id) {
                    Some(DialogTypes::GenericError(_)) => "Error",
                    Some(DialogTypes::FolderAddingErrors(_)) => "Error",
                    None => "What.",
                }
            }
        };

        String::from(title)
    })
    .subscription(OraConverterGui::subscription)
    .run()
}

enum OraConverterGui {
    SetupIssue(SetupState),
    MainWindow(MainState),
}

struct SetupState {
    id: window::Id,
    error: eyre::Report,
}

struct MainState {
    id: window::Id,
    files: BTreeMap<Arc<Path>, FileState>,
    hoster_state: State,
    cancel_conversion: Option<Arc<AtomicBool>>,
    in_progress: Vec<Arc<Path>>,
    errored: Vec<ErroredFile>,
    done: Vec<Arc<Path>>,
    recursive_folders: bool,
    dialog_windows: HashMap<window::Id, DialogTypes>,
}

struct FileState {
    name: Arc<Path>,
    hovered: bool,
}

struct ErroredFile {
    name: Arc<Path>,
    error: Arc<eyre::Report>,
    opened: bool,
}

impl ErroredFile {
    pub fn new(name: Arc<Path>, error: Arc<eyre::Report>) -> Self {
        Self {
            name,
            error,
            opened: false,
        }
    }
}

impl FileState {
    pub fn new(name: Arc<Path>) -> Self {
        Self {
            name,
            hovered: false,
        }
    }
}

enum DialogTypes {
    FolderAddingErrors(SelectFolderErrors),
    GenericError(eyre::Report),
}

#[derive(Debug, Clone)]
enum Message {
    OpenFileDialog,
    AddFiles(Option<Vec<FileHandle>>),
    OpenFolderDialog,
    AddFolder(Option<FileHandle>),
    ToggledRecursion(bool),
    RemoveFile(Arc<Path>),
    WindowClosed(window::Id),
    RetryInit,
    OpenPdnDialog,
    SelectPdn(Option<FileHandle>),
    CopyError(window::Id),
    CopyFileError(Arc<Path>),
    SelectOutput,
    StartConversion(Option<FileHandle>),
    FileConverionStarted(Arc<Path>),
    FileConverted(Arc<Path>, Result<(), Arc<eyre::Report>>),
    CancelConversion,
    ConversionDone,
    CloseDialog(window::Id),
    SetParallelism(NonZeroUsize),
    HoverEvent(Arc<Path>, bool),
    ToggleFileError(Arc<Path>),
}

impl OraConverterGui {
    fn main_window(hoster_state: State) -> (Self, Task<Message>) {
        let (id, t) = window::open(Settings {
            size: Size::new(720., 600.),
            min_size: Some(Size::new(720., 300.)),
            ..Default::default()
        });
        let state = MainState {
            id,
            files: BTreeMap::new(),
            hoster_state,
            cancel_conversion: None,
            recursive_folders: false,
            done: Vec::new(),
            errored: Vec::new(),
            in_progress: Vec::new(),
            dialog_windows: HashMap::new(),
        };
        (Self::MainWindow(state), t.discard())
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        let state = match self {
            Self::MainWindow(state) => state,
            Self::SetupIssue(setup) => {
                return match &message {
                    Message::OpenPdnDialog => window::set_mode(setup.id, window::Mode::Hidden)
                        .chain(Task::perform(
                            rfd::AsyncFileDialog::new()
                                .set_title("Select Paint.net Install")
                                .set_directory(Path::new(DEFAULT_LOCATION).parent().unwrap())
                                .pick_folder(),
                            Message::SelectPdn,
                        )),
                    Message::RetryInit => match State::init() {
                        Ok(hoster_state) => {
                            let (state, t) = Self::main_window(hoster_state);

                            let id = setup.id;
                            *self = state;

                            t.discard().chain(window::close(id))
                        }
                        Err(error) => {
                            setup.error = eyre::Report::new(error);
                            Task::none()
                        }
                    },
                    Message::SelectPdn(None) => window::set_mode(setup.id, window::Mode::Windowed),
                    Message::SelectPdn(Some(path)) => {
                        match State::from_pdn_location(&path.path().join("paintdotnet.dll")) {
                            Ok(hoster_state) => {
                                let (state, t) = Self::main_window(hoster_state);

                                let id = setup.id;
                                *self = state;

                                t.discard().chain(window::close(id))
                            }
                            Err(error) => {
                                setup.error = error;
                                window::set_mode(setup.id, window::Mode::Windowed)
                            }
                        }
                    }
                    Message::WindowClosed(_) => iced::exit(),
                    Message::CopyError(_) => {
                        iced::clipboard::write(append_version(eyre_to_text(&setup.error)))
                    }
                    _ => Task::none(),
                };
            }
        };

        match message {
            Message::OpenFileDialog => {
                return Task::perform(
                    rfd::AsyncFileDialog::new()
                        .set_title("Add Files")
                        .add_filter("Paint.net format", &["pdn"])
                        .pick_files(),
                    Message::AddFiles,
                );
            }
            Message::AddFiles(Some(new)) if state.cancel_conversion.is_none() => {
                state
                    .files
                    .extend(new.iter().map(FileHandle::path).map(|x| {
                        (
                            x.into(),
                            FileState::new(Path::new(x.file_stem().unwrap()).into()),
                        )
                    }));
            }
            Message::OpenFolderDialog => {
                return Task::perform(
                    rfd::AsyncFileDialog::new()
                        .set_title("Add Folder Recursively")
                        .pick_folder(),
                    Message::AddFolder,
                );
            }
            Message::ToggledRecursion(b) => state.recursive_folders = b,
            Message::AddFolder(Some(folder)) => {
                if state.cancel_conversion.is_some() {
                    return Task::none();
                }

                if let Err(errors) = state.add_folder(folder) {
                    return state.dialog_window(DialogTypes::FolderAddingErrors(errors));
                }
            }
            Message::RemoveFile(file) => {
                state.files.remove(&file);

                if let Some((_, s)) = state.files.range_mut(file..).next() {
                    s.hovered = true;
                };
            }
            Message::SelectOutput => {
                return Task::perform(
                    rfd::AsyncFileDialog::new()
                        .set_title("Select Output")
                        .pick_folder(),
                    Message::StartConversion,
                );
            }
            Message::StartConversion(Some(output)) => {
                state.in_progress = Vec::new();
                state.errored = Vec::new();
                state.done = Vec::new();

                let output: Arc<Path> = output.path().into();
                let cancel_conversion = Arc::new(AtomicBool::new(false));
                let hoster = state.hoster_state.pdn_hoster().clone();
                let parallelism = state.hoster_state.parallelism();
                let for_conversion = mem::take(&mut state.files);

                state.cancel_conversion = Some(cancel_conversion.clone());
                let sipper = sipper(move |sender| async move {
                    let iter = for_conversion
                        .into_iter()
                        .zip(iter::repeat((sender, output, hoster, cancel_conversion)))
                        .map(
                            |(
                                (path, FileState { name, .. }),
                                (mut sender, output, hoster, cancelleled),
                            )| async move {
                                if cancelleled.load(Ordering::Acquire) {
                                    return;
                                }

                                sender
                                    .send(Message::FileConverionStarted(name.clone()))
                                    .await;

                                let output = output.join(&name).with_extension("ora");
                                let has_parent = name.parent().is_some();
                                let result = tokio::task::spawn_blocking(move || {
                                    if let Some(parent) = output.parent().filter(|_| has_parent) {
                                        create_dir_all(parent)?;
                                    }
                                    hoster.ora_file_from_pdn(&path, &output)
                                })
                                .await
                                .map_err(eyre::Report::new)
                                .and_then(|x| x.map_err(eyre::Report::new));

                                sender
                                    .send(Message::FileConverted(name, result.map_err(Arc::new)))
                                    .await;
                            },
                        );

                    stream::iter(iter)
                        .buffer_unordered(parallelism.get())
                        .collect::<()>()
                        .await;

                    Message::ConversionDone
                });

                return Task::sip(sipper, identity, identity);
            }
            Message::CancelConversion => {
                if let Some(ref c) = state.cancel_conversion {
                    c.store(true, Ordering::Release)
                }
            }
            Message::FileConverionStarted(a) => {
                state.in_progress.push(a);
            }
            Message::FileConverted(path, result) => {
                let get_pos = |p| Arc::ptr_eq(p, &path);

                state
                    .in_progress
                    .remove(state.in_progress.iter().position(get_pos).unwrap());

                if let Err(error) = result {
                    state.errored.push(ErroredFile::new(path, error));
                } else {
                    state.done.push(path);
                }
            }
            Message::ConversionDone => state.cancel_conversion = None,
            Message::WindowClosed(id) if id == state.id => return iced::exit(),
            Message::CloseDialog(id) => return window::close(id),
            Message::CopyError(id) => {
                let Some(types) = state.dialog_windows.get(&id) else {
                    return Task::none();
                };

                let error = match types {
                    DialogTypes::FolderAddingErrors(error) => error.message(),
                    DialogTypes::GenericError(report) => eyre_to_text(report),
                };

                return clipboard::write(append_version(error));
            }
            Message::CopyFileError(file) => {
                if let Some(file) = state
                    .errored
                    .iter_mut()
                    .find(|f| Arc::ptr_eq(&f.name, &file))
                {
                    return clipboard::write(append_version(eyre_to_text(&file.error)));
                }
            }
            Message::SetParallelism(new) => {
                if let Err(error) = state.hoster_state.set_parallelism(new) {
                    return state.dialog_window(DialogTypes::GenericError(error));
                }
            }
            Message::HoverEvent(path, hovered_new) => {
                if let Some(FileState { hovered, .. }) = state.files.get_mut(&path) {
                    *hovered = hovered_new;
                }
            }
            Message::ToggleFileError(file) => {
                if let Some(file) = state
                    .errored
                    .iter_mut()
                    .find(|f| Arc::ptr_eq(&f.name, &file))
                {
                    file.opened = !file.opened;
                }
            }
            _ => (),
        }

        Task::none()
    }

    fn view(&self, id: window::Id) -> Element<'_, Message> {
        let state = match self {
            OraConverterGui::MainWindow(state) => state,
            OraConverterGui::SetupIssue(setup) => return setup.view(),
        };

        if state.id != id {
            let Some(dialog) = state.dialog_windows.get(&id) else {
                return Text::new("You shouldn't be seeing this!").into();
            };

            return match dialog {
                DialogTypes::FolderAddingErrors(errors) => errors.view(id),
                DialogTypes::GenericError(report) => Dialog {
                    header: "Error Occured!",
                    message: eyre_to_text(report),
                    button_left: Some(Button::new("Copy Error").on_press(Message::CopyError(id))),
                    button_right_secondary: None,
                    button_right_most: Some(Button::new("Ok").on_press(Message::CloseDialog(id))),
                },
            }
            .view();
        }

        let files = Column::with_children(
            state
                .files
                .iter()
                .map(|(path, FileState { name, hovered })| {
                    MouseArea::new(file_scrollarea(
                        Container::new(
                            Rich::with_spans([Span::<()>::new(name.to_string_lossy())
                                .strikethrough(*hovered)
                                .font_maybe(hovered.then_some(Font {
                                    style: font::Style::Italic,
                                    ..Font::default()
                                }))])
                            .wrapping(text::Wrapping::None)
                            .height(20),
                        ),
                        |_| container::Style::default(),
                    ))
                    .interaction(Interaction::Pointer)
                    .on_press(Message::RemoveFile(path.clone()))
                    .on_move(|_| Message::HoverEvent(path.clone(), true))
                    .on_exit(Message::HoverEvent(path.clone(), false))
                })
                .map(Element::from),
        );

        let output = state
            .in_progress
            .iter()
            .rev()
            .map(|n| file_text(n))
            .map(Container::new)
            .map(|f| {
                Row::new()
                    .push(file_scrollarea(f, |_| container::Style {
                        // text_color: Some(t.extended_palette().primary.weak.text),
                        ..Default::default()
                    }))
                    .push(
                        Container::new(
                            Svg::new(icons::HOURGLASS.clone())
                                .height(22)
                                .width(22)
                                .style(move |t: &Theme, _| svg::Style {
                                    color: Some(t.extended_palette().primary.strong.color),
                                }),
                        )
                        .padding(Padding::ZERO.left(2).right(12)),
                    )
                    .align_y(Vertical::Center)
            })
            .map(Element::from);

        let output = output.chain(
            state
                .errored
                .iter()
                .map(|file| {
                    Column::new()
                        .push(
                            MouseArea::new(
                                Row::new()
                                    .push(file_scrollarea(
                                        Container::new(file_text(&file.name)),
                                        |_| container::Style {
                                            // background: Some(Background::Color(t.extended_palette().danger.base.color)),
                                            // text_color: Some(t.extended_palette().danger.base.text),
                                            ..Default::default()
                                        },
                                    ))
                                    .push(
                                        Container::new(
                                            Svg::new(icons::ERROR.clone())
                                                .height(22)
                                                .width(22)
                                                .style(move |t: &Theme, _| svg::Style {
                                                    color: Some(
                                                        t.extended_palette().danger.strong.color,
                                                    ),
                                                }),
                                        )
                                        .padding(Padding::ZERO.left(2).right(12)),
                                    )
                                    .align_y(Vertical::Center),
                            )
                            .interaction(Interaction::Pointer)
                            .on_press(Message::ToggleFileError(file.name.clone())),
                        )
                        .push_maybe(file.opened.then(|| {
                            Stack::new()
                                .push(
                                    Container::new(
                                        Scrollable::with_direction(
                                            Text::new(eyre_to_text(&file.error))
                                                .size(14)
                                                .width(Length::Fill),
                                            Direction::Vertical(Scrollbar::new().scroller_width(5)),
                                        )
                                        .style(
                                            |t: &Theme, s| {
                                                let default = scrollable::Catalog::style(
                                                    t,
                                                    &<Theme as scrollable::Catalog>::default(),
                                                    s,
                                                );
                                                scrollable::Style {
                                                    vertical_rail: Rail {
                                                        background: None,
                                                        border: Border::default(),
                                                        scroller: default.vertical_rail.scroller,
                                                    },
                                                    ..default
                                                }
                                            },
                                        ),
                                    )
                                    .padding(Padding::ZERO.left(10).right(14))
                                    .max_height(120),
                                )
                                .push(
                                    Container::new(
                                        MouseArea::new(
                                            Svg::new(icons::CLIPBOARD.clone())
                                                .width(25)
                                                .height(25)
                                                .style(move |t: &Theme, _| svg::Style {
                                                    color: Some(t.palette().text),
                                                }),
                                        )
                                        .on_press(Message::CopyFileError(file.name.clone()))
                                        .interaction(Interaction::Pointer),
                                    )
                                    .align_right(Length::Fill)
                                    .padding(Padding::ZERO.right(24))
                                    .align_x(Horizontal::Right),
                                )
                        }))
                })
                .map(Element::from),
        );

        let output = output.chain(
            state
                .done
                .iter()
                .rev()
                .map(|n| file_text(n))
                .map(Container::new)
                .map(|f| {
                    file_scrollarea(f, |_| container::Style {
                        // background: Some(Background::Color(
                        //     t.extended_palette().success.base.color,
                        // )),
                        // text_color: Some(t.extended_palette().success.base.text),
                        ..Default::default()
                    })
                })
                .map(Element::from),
        );

        let output = Column::with_children(output);

        let output = Container::new(output).width(Length::FillPortion(5));
        let filelist = Container::new(files).width(Length::FillPortion(5));

        let range = (1..=state.hoster_state.max_parallelism().get()).collect::<Vec<usize>>();

        let picklist = tooltip(
            pick_list(range, Some(state.hoster_state.parallelism().get()), |x| {
                Message::SetParallelism(unsafe { NonZeroUsize::new_unchecked(x) })
            }),
            tooltip_element("Sets how many files are converted at once"),
            tooltip::Position::Top,
        );

        let working = state.cancel_conversion.is_some();
        let rest = Column::with_children([
            Row::with_children([
                button("Add Files")
                    .on_press_maybe(working.not().then_some(Message::OpenFileDialog))
                    .into(),
                button("Add Folder")
                    .on_press_maybe(working.not().then_some(Message::OpenFolderDialog))
                    .into(),
                tooltip(
                    checkbox("Recursive", state.recursive_folders)
                        .on_toggle(Message::ToggledRecursion),
                    tooltip_element("Adds all folders in the selected folder"),
                    tooltip::Position::Bottom,
                )
                .into(),
            ])
            .align_y(Alignment::Center)
            .spacing(8)
            .into(),
            row![
                button("Convert").on_press_maybe(
                    (!state.files.is_empty() && !working).then_some(Message::SelectOutput),
                ),
                button("Cancel").on_press_maybe(working.then_some(Message::CancelConversion)),
            ]
            .spacing(8)
            .into(),
            picklist.into(),
            Container::new(
                Text::new(VERSION)
                    .color(Color::from_rgb8(128, 128, 128))
                    .size(15),
            )
            .align_bottom(Length::Fill)
            .align_right(Length::Fill)
            .padding(Padding::ZERO.right(9))
            .into(),
        ])
        .spacing(8)
        .width(Length::FillPortion(4))
        .padding(Padding::ZERO.top(8).bottom(8));

        let mut row = Row::new();
        let filelist = if !state.files.is_empty() {
            filelist
        } else {
            output
        };
        row = row.push(scrollable(filelist).style(|t: &Theme, s| {
            let default =
                scrollable::Catalog::style(t, &<Theme as scrollable::Catalog>::default(), s);
            scrollable::Style {
                vertical_rail: Rail {
                    background: None,
                    border: Border::default(),
                    scroller: default.vertical_rail.scroller,
                },
                ..default
            }
        }));
        row = row.push(rest);

        row.spacing(8.0).into()
    }

    fn subscription(&self) -> Subscription<Message> {
        window::close_events().map(Message::WindowClosed)
    }
}

fn tooltip_element(text: &str) -> Container<'_, Message> {
    Container::new(text)
        .padding(Padding::new(4.0))
        .style(|t: &Theme| background(Background::Color(t.palette().background)))
}

fn file_text(file: &Path) -> Text<'_> {
    Text::new(file.to_string_lossy())
        .wrapping(text::Wrapping::None)
        .height(20)
        .shaping(text::Shaping::Advanced)
}

fn file_scrollarea<F>(file: Container<'_, Message>, styling: F) -> Scrollable<'_, Message>
where
    F: Fn(&Theme) -> container::Style,
    F: 'static,
{
    Scrollable::with_direction(
        file.padding(Padding::ZERO.left(8).right(35))
            .align_y(Vertical::Center),
        Direction::Horizontal(Scrollbar::new().scroller_width(30.)),
    )
    .height(30.)
    .width(Length::Fill)
    .style(move |t: &Theme, s| scrollable::Style {
        container: styling(t),
        horizontal_rail: Rail {
            background: None,
            border: Border::default(),
            scroller: Scroller {
                color: Color::TRANSPARENT,
                border: Border::default(),
            },
        },
        // container: iced::widget::container::Style { background: Some(Background::Color(Color::from_rgb8(255, 255, 255))),..Default::default() },
        ..scrollable::Catalog::style(t, &<Theme as scrollable::Catalog>::default(), s)
    })
}

impl MainState {
    pub fn dialog_window(&mut self, error: DialogTypes) -> Task<Message> {
        let (id, task) = window::open(Settings {
            position: Position::Centered,
            size: ERROR_WINDOW_SIZE,
            resizable: false,
            ..Default::default()
        });

        self.dialog_windows.insert(id, error);

        task.discard()
    }

    pub fn add_folder(&mut self, folder: FileHandle) -> Result<(), SelectFolderErrors> {
        let same_components = folder.path().iter().count();

        let mut result = Ok(());

        if !self.recursive_folders {
            let mut i = 0;

            for res in read_dir(folder.path())? {
                i += 1;
                if i == MAX_FILE_SEARCH {
                    return Err(FolderSearchError::TooLong.into());
                }

                let entry = match res.and_then(|x| {
                    let r = x.metadata();
                    r.map(|m| (x, m.is_file()))
                }) {
                    Ok((entry, true)) => entry,
                    Ok((_, false)) => continue,
                    Err(error) => {
                        result = SelectFolderErrors::result_accumulate(result, error.into());
                        continue;
                    }
                };

                let path = entry.path();

                if path.extension().is_none_or(|x| x != "pdn") {
                    continue;
                }

                let mut short_form = PathBuf::from(entry.file_name());

                short_form.set_extension("");
                self.files
                    .insert(path.into(), FileState::new(short_form.into()));
            }
        } else {
            let mut i = 0;

            for res in walkdir::WalkDir::new(folder.path()).sort_by_file_name() {
                i += 1;
                if i == MAX_FILE_SEARCH {
                    return Err(FolderSearchError::TooLong.into());
                }

                let entry = match res {
                    Ok(e) => e,
                    Err(error) => {
                        result = SelectFolderErrors::result_accumulate(result, error.into());
                        continue;
                    }
                };

                if entry.file_type().is_file().not()
                    || entry.path().extension().is_none_or(|x| x != "pdn")
                {
                    continue;
                }

                let mut short_form = entry
                    .path()
                    .iter()
                    .skip(same_components)
                    .collect::<PathBuf>();

                short_form.set_extension("");

                self.files
                    .insert(entry.into_path().into(), FileState::new(short_form.into()));
            }
        };

        result
    }
}

impl SetupState {
    pub fn view(&self) -> Element<'_, Message> {
        let default_not_found = self
            .error
            .downcast_ref::<StateInitError>()
            .is_some_and(|e| matches!(e, StateInitError::DefaultNotFound));

        let header = if default_not_found {
            "Additional Setup Required"
        } else {
            "Setup Error Occured"
        };

        Dialog {
            header,
            message: eyre_to_text(&self.error),
            button_left: default_not_found
                .not()
                .then_some(button("Copy Error").on_press(Message::CopyError(self.id))),
            button_right_secondary: Some(button("Retry").on_press(Message::RetryInit)),
            button_right_most: Some(button("Select Manually").on_press(Message::OpenPdnDialog)),
        }
        .view()
    }
}

fn eyre_to_text(report: &eyre::Report) -> String {
    let mut output = String::new();

    write!(&mut output, "{report}").unwrap();

    if let Some(cause) = report.source() {
        output.push_str("\n\nError stack:");
        for (i, error) in std::iter::successors(Some(cause), |&e| e.source()).enumerate() {
            writeln!(&mut output, "{i}: {error}").unwrap();
        }
    }

    output
}

fn append_version(msg: String) -> String {
    msg + "\n\n" + VERSION
}
