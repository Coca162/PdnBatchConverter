use std::{fmt::Write, io};

use iced::{widget::Button, window};

use crate::{MAX_FILE_SEARCH, Message, dialog::Dialog};
use pdn_conv::VERSION;

#[derive(Debug, thiserror::Error)]
pub enum FolderSearchError {
    #[error(transparent)]
    Io(#[from] io::Error),
    #[error(transparent)]
    WalkDir(#[from] walkdir::Error),
    #[error(
        "Folder selection ended early due to over {MAX_FILE_SEARCH} files being searched. Likely not every .pdn file was found!"
    )]
    TooLong,
}

#[derive(Debug)]
pub enum SelectFolderErrors {
    Singular(FolderSearchError),
    Multiple(Vec<FolderSearchError>),
}

impl SelectFolderErrors {
    pub fn add_error(self, error: FolderSearchError) -> Self {
        match (self, error) {
            (_, FolderSearchError::TooLong) => Self::Singular(FolderSearchError::TooLong),
            (Self::Singular(single), new) => Self::Multiple(vec![single, new]),
            (Self::Multiple(mut multiple), error) => {
                multiple.push(error);
                Self::Multiple(multiple)
            }
        }
    }

    pub fn result_accumulate(
        result: Result<(), Self>,
        error: FolderSearchError,
    ) -> Result<(), Self> {
        match result {
            Ok(()) => Err(Self::Singular(error)),
            Err(p) => Err(p.add_error(error)),
        }
    }

    pub fn view(&self, id: window::Id) -> Dialog {
        let copy_text = match self {
            Self::Singular(_) => "Copy Error",
            Self::Multiple(_) => "Copy Errors",
        };

        Dialog {
            header: "Couldn't search all files",
            message: self.message(),
            button_left: Some(Button::new(copy_text).on_press(Message::CopyError(id))),
            button_right_secondary: None,
            button_right_most: Some(Button::new("Ok").on_press(Message::CloseDialog(id))),
        }
    }

    pub fn message(&self) -> String {
        match self {
            Self::Singular(e) => format!("{e}\n{VERSION}"),
            Self::Multiple(e) => {
                let mut output = String::from("Multiple errors from searching found:");
                for e in e {
                    write!(&mut output, "\n- {e}").unwrap();
                }
                output
            }
        }
    }
}

impl From<io::Error> for SelectFolderErrors {
    fn from(value: io::Error) -> Self {
        Self::Singular(FolderSearchError::Io(value))
    }
}

impl From<FolderSearchError> for SelectFolderErrors {
    fn from(value: FolderSearchError) -> Self {
        Self::Singular(value)
    }
}

impl From<walkdir::Error> for SelectFolderErrors {
    fn from(value: walkdir::Error) -> Self {
        Self::Singular(FolderSearchError::WalkDir(value))
    }
}
