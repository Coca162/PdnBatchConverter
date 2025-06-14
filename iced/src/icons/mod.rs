// Icons are from the Lucide project licensed under ISC :3
// https://lucide.dev/license

use iced::advanced::svg::Handle;
use std::{borrow::Cow, sync::LazyLock};

pub static CLIPBOARD: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_memory(Cow::Borrowed(include_bytes!("clipboard.svg").as_slice()))
});

pub static DELETE: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(Cow::Borrowed(include_bytes!("delete.svg").as_slice())));

pub static ERROR: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(Cow::Borrowed(include_bytes!("error.svg").as_slice())));

pub static HOURGLASS: LazyLock<Handle> = LazyLock::new(|| {
    Handle::from_memory(Cow::Borrowed(include_bytes!("hourglass.svg").as_slice()))
});

pub static TRASH: LazyLock<Handle> =
    LazyLock::new(|| Handle::from_memory(Cow::Borrowed(include_bytes!("trash.svg").as_slice())));
