use iced::{widget::{button, column, text}, Element, Length};

#[derive(Debug, Default)]
struct Counter {
    value: i8
}

#[derive(Debug, Clone, Copy)]
enum Message {
    Increment,
    Decrement
}

impl Counter {
    fn update(&mut self, message: Message) {
        match message {
            Message::Increment => self.value += 1,
            Message::Decrement => self.value +=-1
        }
    }

    fn view(&self) -> Element<'_, Message>{
        let incrementator = button(text("+").width(Length::Fill).center()).on_press(Message::Increment)
            .width(Length::Fill)
            .height(Length::Fill);
        let decrementator = button(text("+").width(Length::Fill).center()).on_press(Message::Decrement)
            .width(Length::Fill)
            .height(Length::Fill);
        let counter = text(self.value)
            .width(Length::Fill)
            .height(Length::Fill)
            .center();

        let interface = column![incrementator, counter, decrementator]
            .spacing(10)
            .padding(10);
        interface.into()
    }
}

fn main() -> iced::Result {
    iced::run("COUNTER !", Counter::update, Counter::view)
}
