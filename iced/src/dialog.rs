use iced::{
    Element, Length, Padding,
    widget::{Button, Container, Row, Text, column, scrollable, text},
};

use crate::Message;

pub struct Dialog {
    pub header: &'static str,
    pub message: String,
    pub button_left: Option<Button<'static, Message>>,
    pub button_right_secondary: Option<Button<'static, Message>>,
    pub button_right_most: Option<Button<'static, Message>>,
}

impl Dialog {
    pub fn view(self) -> Element<'static, Message> {
        column![
            Container::new(Text::new(self.header).size(20)).center_x(Length::Fill),
            scrollable(
                Container::new(Text::new(self.message).wrapping(text::Wrapping::WordOrGlyph))
                    .padding(Padding::ZERO.right(14.))
            )
            .width(Length::Fill)
            .height(Length::Fill),
            Container::new(
                Row::with_capacity(2)
                    .push(
                        self.button_left
                            .map(|b| Container::new(b).align_left(Length::Fill)),
                    )
                    .push(
                        Container::new(
                            Row::with_capacity(2)
                                .push(self.button_right_secondary)
                                .push(self.button_right_most)
                                .spacing(6),
                        )
                        .align_right(Length::Fill),
                    )
            )
            .align_bottom(Length::Fill)
            .height(Length::Shrink)
        ]
        .spacing(5.0)
        .padding(Padding::new(6.).top(0))
        .into()
    }
}
