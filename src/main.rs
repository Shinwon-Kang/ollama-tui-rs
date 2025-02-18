use std::{net::Shutdown, process::exit};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Flex, Layout, Position, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
    },
    DefaultTerminal, Frame,
};
use serde::{Deserialize, Serialize};

struct App {
    exit: bool,
    models_info: ModelInfo,
    ollama_api: OllamaApi,

    input: String,
    input_mode: InputMode,
    character_index: usize,
    chat_log: Vec<String>,
}

enum InputMode {
    Normal,
    Editing,
}

struct ModelInfo {
    models: ModelList,
    selected_model: ListState,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModelList {
    models: Vec<Model>,
}

#[derive(Serialize, Deserialize, Debug)]
struct Model {
    name: String,
    model: String,
    modified_at: String,
    size: u64,
    digest: String,
    details: ModelDetails,
}

#[derive(Serialize, Deserialize, Debug)]
struct ModelDetails {
    format: String,
    family: String,
    families: Option<Vec<String>>,
    parameter_size: String,
    quantization_level: String,
}

struct OllamaApi {
    base_url: String,
}

impl Default for OllamaApi {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
        }
    }
}

impl OllamaApi {
    async fn get_models(&self) -> Result<ModelList, reqwest::Error> {
        let response = reqwest::get(format!("{}/api/tags", self.base_url)).await;
        match response {
            Ok(response) => {
                let response = response.bytes().await.unwrap();
                let model_list = serde_json::from_slice::<ModelList>(&response).unwrap();
                Ok(model_list)
            }
            Err(error) => Err(error),
        }
    }

    // TODO
    async fn generate(&self, prompt: String) -> Result<String, reqwest::Error> {
        Ok("".to_string())
    }
}

impl Default for ModelInfo {
    fn default() -> Self {
        Self {
            models: ModelList { models: vec![] },
            selected_model: ListState::default(),
        }
    }
}

impl Default for App {
    fn default() -> Self {
        Self {
            exit: false,
            ollama_api: OllamaApi::default(),
            models_info: ModelInfo::default(),
            input: String::new(),
            input_mode: InputMode::Normal,
            character_index: 0,
            chat_log: Vec::new(),
        }
    }
}

// App logic
impl App {
    async fn run(mut self, mut terminal: DefaultTerminal) -> std::io::Result<()> {
        self.load_models().await;

        while !self.exit {
            terminal.draw(|frame| self.draw(frame))?;
            if let Event::Key(key) = event::read()? {
                self.handle_key(key);
            }
        }

        Ok(())
    }

    fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Esc => self.exit = true,
                KeyCode::Char('e') => self.input_mode = InputMode::Editing, // TODO: change to input mode
                KeyCode::Down => self.models_info.selected_model.select_next(),
                KeyCode::Up => self.models_info.selected_model.select_previous(),
                KeyCode::Enter => self.select_model(),
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter => self.chat_message(),
                KeyCode::Char(c) => self.update_input(c),
                KeyCode::Backspace => self.delete_input(),
                KeyCode::Left => self.move_cursor_left(),
                KeyCode::Right => self.move_cursor_right(),
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                _ => {}
            },
        }
    }

    fn select_model(&mut self) {
        if let Some(selected_model) = self.models_info.selected_model.selected() {
            println!(
                "Selected model: {}",
                self.models_info.models.models[selected_model].name
            );
        }
    }

    fn chat_message(&mut self) {
        self.chat_log.push(self.input.clone());
        self.input.clear();
        self.reset_cursor();
    }

    fn get_character_index(&self) -> usize {
        self.input
            .char_indices()
            .map(|(i, _)| i)
            .nth(self.character_index)
            .unwrap_or(self.input.len())
    }

    fn move_cursor_left(&mut self) {
        let cursor_moved_left = self.character_index.saturating_sub(1);
        self.character_index = self.clamp_cursor(cursor_moved_left);
    }

    fn move_cursor_right(&mut self) {
        let cursor_moved_right = self.character_index.saturating_add(1);
        self.character_index = self.clamp_cursor(cursor_moved_right);
    }

    fn clamp_cursor(&self, cursor_index: usize) -> usize {
        cursor_index.clamp(0, self.input.chars().count())
    }

    fn reset_cursor(&mut self) {
        self.character_index = 0;
    }

    fn update_input(&mut self, c: char) {
        let index = self.get_character_index();
        self.input.insert(index, c);
        self.move_cursor_right();
    }

    fn delete_input(&mut self) {
        let is_not_cursor_leftmost = self.character_index != 0;
        if is_not_cursor_leftmost {
            let current_index = self.character_index;
            let from_left_to_current_index = current_index - 1;

            let before_char_to_delete = self.input.chars().take(from_left_to_current_index);
            let after_char_to_delete = self.input.chars().skip(current_index);

            self.input = before_char_to_delete.chain(after_char_to_delete).collect();
            self.move_cursor_left();
        }
    }
}

// UI
impl App {
    fn draw(&mut self, frame: &mut Frame) {
        let [header_area, list_area, footer_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Fill(3),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        // TODO: length should be dynamic based on the number of models
        let [header_area] = Layout::vertical([Constraint::Length(7)])
            .flex(Flex::Center)
            .areas(header_area);

        // TODO: separate input area
        let [list_area, chat_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Fill(4)]).areas(list_area);

        let [chat_area, input_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(3)]).areas(chat_area);

        // TODO: length should be dynamic based on the number of models
        let [list_area] = Layout::horizontal([Constraint::Length(15)])
            .flex(Flex::Center)
            .areas(list_area);
        let [list_area] = Layout::vertical([Constraint::Length(2)])
            .flex(Flex::Center)
            .areas(list_area);

        self.render_header(frame, header_area);
        self.render_model_list(frame, list_area);
        self.render_chat(frame, chat_area);
        self.render_text_input(frame, input_area);
        self.render_helper(frame, footer_area);

        match self.input_mode {
            InputMode::Normal => {}
            InputMode::Editing => frame.set_cursor_position(Position::new(
                input_area.x + self.character_index as u16 + 1,
                input_area.y + 1,
            )),
        }
    }

    fn render_header(&mut self, frame: &mut Frame, area: Rect) {
        let header = Paragraph::new(
            r"
       _ _                             _         _                
  ___ | | | __ _ _ __ ___   __ _      | |_ _   _(_)      _ __ ___ 
 / _ \| | |/ _` | '_ ` _ \ / _` |_____| __| | | | |_____| '__/ __|
| (_) | | | (_| | | | | | | (_| |_____| |_| |_| | |_____| |  \__ \
 \___/|_|_|\__,_|_| |_| |_|\__,_|      \__|\__,_|_|     |_|  |___/",
        )
        .centered();
        frame.render_widget(header, area);
    }

    fn render_model_list(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .models_info
            .models
            .models
            .iter()
            .map(|model| ListItem::new(format!("● {}", model.name)))
            .collect();

        let list = List::new(items)
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        frame.render_stateful_widget(list, area, &mut self.models_info.selected_model);
    }

    fn render_chat(&mut self, frame: &mut Frame, area: Rect) {
        let chat_log: Vec<ListItem> = self
            .chat_log
            .iter()
            .enumerate()
            .map(|(i, c)| {
                let content = Line::from(Span::raw(format!("{i}: {c}")));
                ListItem::new(content)
            })
            .collect();

        let chat = List::new(chat_log).block(Block::bordered().title("Chat"));

        frame.render_widget(chat, area);
    }

    fn render_text_input(&mut self, frame: &mut Frame, area: Rect) {
        let input = Paragraph::new(self.input.as_str())
            .style(match self.input_mode {
                InputMode::Normal => Style::default(),
                InputMode::Editing => Style::default().fg(Color::Yellow),
            })
            .block(Block::bordered().title("Input"));

        frame.render_widget(input, area);
    }

    fn render_helper(&mut self, frame: &mut Frame, area: Rect) {
        let text = "▲ ▼: select, Enter: choose, q: quit";
        let helper = Paragraph::new(text).centered();

        frame.render_widget(helper, area);
    }
}

// Ollama API
impl App {
    async fn load_models(&mut self) {
        let models = self.ollama_api.get_models().await;
        match models {
            Ok(models) => {
                self.models_info.models = models;
            }
            Err(error) => {
                eprintln!("Error loading models: {}", error);
                // TODO: show error message on tui
            }
        }
    }
}

fn shutdown() -> std::io::Result<()> {
    crossterm::execute!(std::io::stderr(), crossterm::terminal::LeaveAlternateScreen)?;
    crossterm::terminal::disable_raw_mode()?;
    Ok(())
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let terminal = ratatui::init();

    App::default().run(terminal).await?;
    shutdown()?;

    Ok(())
}
