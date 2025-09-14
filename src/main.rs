use std::collections::VecDeque;
use std::env::current_dir;
use std::io;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use iced::widget::{button, column, container, horizontal_space, row, text, text_editor};
use iced::{Element, Font, Length, Task};
use rfd::{AsyncFileDialog, FileHandle};

#[derive(Debug, Default)]
struct State {
    content: text_editor::Content,
    file_path: Option<PathBuf>,
    prev_path: PathBuf,
    error: VecDeque<String>,
}

#[derive(Debug, Clone)]
enum Message {
    Edit(text_editor::Action),
    FileOpened(Arc<io::Result<String>>, PathBuf),
    OpenFileDialog,
    OpenFile(Option<FileHandle>),
    RemoveError,
    New,
    Save,
    SaveAs,
    SavedFile(Option<Arc<io::Result<PathBuf>>>),
}

impl State {
    fn set_file_path(&mut self, path: Option<PathBuf>) {
        match path.clone() {
            Some(p) => {
                self.prev_path = if p.is_dir() {
                    p
                } else {
                    p.parent()
                        .map(|p| p.to_path_buf())
                        .unwrap_or(current_dir().unwrap_or_default())
                }
            }
            None => (),
        };
        self.file_path = path
    }
}

impl State {
    fn new() -> (Self, Task<Message>) {
        (
            Self {
                content: text_editor::Content::new(),
                file_path: None,
                prev_path: PathBuf::new(),
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
                    self.set_file_path(Some(path));
                    Task::none()
                }
                Err(error) => self.add_error(format!("Could not open file: {error}")),
            },
            Message::OpenFileDialog => Task::perform(
                file_select_win_builder(
                    "Open file ...",
                    self.prev_path.clone(),
                    file_name_opt(self.file_path.as_ref()),
                )
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
            Message::New => {
                self.content = text_editor::Content::new();
                self.set_file_path(None);
                Task::none()
            }
            Message::Save => {
                let text = self.content.text();
                Task::perform(
                    save_file(self.file_path.clone(), self.prev_path.clone(), text),
                    Message::SavedFile,
                )
            }
            Message::SaveAs => {
                let text = self.content.text();
                Task::perform(
                    save_file_as(
                        self.prev_path.clone(),
                        text,
                        file_name_opt(self.file_path.as_ref()),
                    ),
                    Message::SavedFile,
                )
            }
            Message::SavedFile(data) => match data {
                Some(result) => match &*result {
                    Ok(path) => {
                        self.set_file_path(Some(path.clone()));
                        self.add_error(format!("{} saved!", path.to_str().unwrap_or("file")))
                    }
                    Err(error) => self.add_error(format!("Could not save file: {error}")),
                },
                None => self.add_error("File save aborted. File not saved.".to_string()),
            },
        }
    }

    fn view(&self) -> Element<'_, Message> {
        let menu = self.view_menu();

        let placeholder = match self.file_path {
            Some(_) => "Type here ...",
            None => "Welcome! Open a file or start typing here ...",
        };
        let editor = text_editor(&self.content)
            .placeholder(placeholder)
            .on_action(Message::Edit)
            .height(Length::Fill);

        let bottom_info = self.view_bottom_info();

        container(column![menu, editor, bottom_info].spacing(10))
            .padding(10)
            .into()
    }

    fn view_menu(&self) -> Element<'_, Message> {
        let new_button = button(icon(Icon::NewFile)).on_press(Message::New);
        let open_button = button(icon(Icon::File)).on_press(Message::OpenFileDialog);
        let save_button = button(icon(Icon::Save)).on_press(Message::Save);
        //let save_as_button = button("save as").on_press(Message::SaveAs);

        row![new_button, open_button, save_button/*, save_as_button*/]
            .spacing(10)
            .into()
    }

    fn view_bottom_info(&self) -> Element<'_, Message> {
        let (line, column) = self.content.cursor_position();
        let cursor_position = text(format!("Line: {}, Column: {}", line + 1, column + 1));

        let path = self
            .file_path
            .as_deref()
            .map(Path::to_str)
            .unwrap_or(Some("No file yet, please save or open file :)"))
            .unwrap_or("Can't display path :(");

        let error = self.error.front().map(String::as_str).unwrap_or(path);

        row![text(error), horizontal_space(), cursor_position].into()
    }

    fn add_error(&mut self, text: String) -> Task<Message> {
        self.error.push_back(text);
        Task::perform(tokio::time::sleep(Duration::from_secs(4)), |_| {
            Message::RemoveError
        })
    }
}
async fn load_file(path: impl AsRef<Path>) -> (Arc<io::Result<String>>, PathBuf) {
    let buf = path.as_ref().to_path_buf();
    (Arc::new(tokio::fs::read_to_string(path).await), buf)
}

async fn save_file(
    file_path: Option<impl AsRef<Path>>,
    root_search_path: impl AsRef<Path>,
    text: impl AsRef<[u8]>,
) -> Option<Arc<io::Result<PathBuf>>> {
    match file_path {
        Some(path) => {
            let path_buf = path.as_ref().to_path_buf();
            let write = tokio::fs::write(path, text).await;
            Some(Arc::new(write.map(|_| path_buf)))
        }
        None => Box::pin(save_file_as(root_search_path, text, None::<String>)).await,
    }
}

async fn save_file_as(
    file_path: impl AsRef<Path>,
    text: impl AsRef<[u8]>,
    file_name: Option<impl Into<String>>,
) -> Option<Arc<io::Result<PathBuf>>> {
    let path_opt = file_select_win_builder("Save as ...", &file_path, file_name)
        .save_file()
        .await;

    match path_opt {
        Some(handle) => save_file(Some(handle.path().to_path_buf()), file_path, text).await,
        None => None,
    }
}

fn file_select_win_builder(
    title: impl Into<String>,
    path: impl AsRef<Path>,
    file_name: Option<impl Into<String>>,
) -> AsyncFileDialog {
    let default_path = current_dir().unwrap_or_default();
    let result = rfd::AsyncFileDialog::new()
        .set_directory({
            let path = path.as_ref().to_path_buf();
            if path.is_dir() {
                path
            } else {
                path.parent()
                    .map(|p| p.to_path_buf())
                    .unwrap_or(default_path)
            }
        })
        .set_title(title);
    match file_name {
        Some(name) => result.set_file_name(name),
        None => result,
    }
}

fn file_name_opt(path: Option<impl AsRef<Path>>) -> Option<String> {
    path?
        .as_ref()
        .to_path_buf()
        .file_name()?
        .to_os_string()
        .into_string()
        .ok()
}

enum Icon {
    File,
    NewFile,
    Save
}

fn icon<'a>(icon: Icon) -> Element<'a, Message> {
    const FONT: Font = Font::with_name("icons-font");

    let code = match icon {
        Icon::File => "\u{E802}",
        Icon::NewFile => "\u{E803}",
        Icon::Save => "\u{E801}"
    };

    text(code).font(FONT).into()
}

fn main() -> iced::Result {
    let mut icon_font_path = PathBuf::new();
    icon_font_path.push("fonts");
    icon_font_path.push("icons-font.ttf");

    iced::application("first-app", State::update, State::view)
        .font(std::fs::read(icon_font_path).unwrap())
        .run_with(State::new)
}
