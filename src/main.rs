use std::collections::VecDeque;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use iced::widget::{button, column, container, horizontal_space, row, text, text_editor};
use iced::{Element, Length, Task};
use rfd::FileHandle;

#[derive(Debug, Default)]
struct State {
    content: text_editor::Content,
    file_path: Option<PathBuf>,
    error: VecDeque<String>,
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    FileOpened(Arc<io::Result<String>>, PathBuf),
    OpenFileDialog,
    OpenFile(Option<FileHandle>),
    RemoveError,
}

impl State {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                content: text_editor::Content::new(),
                file_path: None,
                error: VecDeque::new(),
            },
            Task::none(),
        )
    }

    fn update(&mut self, message: Message) -> Task<Message> {
        match message {
            Message::Edit(action) => {
                self.content.perform(action);
                Task::none()
            }
            Message::FileOpened(result, path) => match &*result {
                Ok(text) => {
                    self.content = text_editor::Content::with_text(text);
                    self.file_path = Some(path);
                    Task::none()
                }
                Err(error) => {
                    self.error
                        .push_back(format!("Could not open file: {error}"));
                    Task::perform(tokio::time::sleep(Duration::from_secs(5)), |_| {
                        Message::RemoveError
                    })
                }
            },
            Message::OpenFileDialog => Task::perform(
                rfd::AsyncFileDialog::new()
                    .set_title("select text file")
                    .set_directory(std::env::current_dir().unwrap_or_default())
                    .pick_file(),
                Message::OpenFile,
            ),
            Message::OpenFile(file_handle_opt) => match file_handle_opt {
                Some(handle) => {
                    Task::perform(load_file(handle.path().to_path_buf()), |(res, buf)| {
                        Message::FileOpened(res, buf)
                    })
                }
                None => Task::none(),
            },
            Message::RemoveError => {
                self.error.pop_front();
                Task::none()
            }
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let menu = button("open").on_press(Message::OpenFileDialog);

        let placeholder = match self.file_path {
            Some(_) => "Type here ...",
            None => "Welcome! Open a file or start typing here ...",
        };
        let editor = text_editor(&self.content)
            .placeholder(placeholder)
            .on_action(Message::Edit)
            .height(Length::Fill);

        let bottom_info = {
            let (line, column) = self.content.cursor_position();
            let cursor_position = text(format!("Line: {}, Column: {}", line + 1, column + 1));

            let path = self
                .file_path
                .as_deref()
                .map(Path::to_str)
                .unwrap_or(Some("No file yet, please save or open file :)"))
                .unwrap_or("Can't display path :(");

            let error = self.error.front().map(String::as_str).unwrap_or(path);
 
            row![text(error), horizontal_space(), cursor_position]
        };

        container(column![menu, editor, bottom_info].spacing(10))
            .padding(10)
            .into()
    }
}

async fn load_file(path: impl AsRef<Path>) -> (Arc<io::Result<String>>, PathBuf) {
    let buf = path.as_ref().to_path_buf();
    (Arc::new(tokio::fs::read_to_string(path).await), buf)
}

fn main() -> iced::Result {
    iced::application("first-app", State::update, State::view).run_with(State::new)
}
