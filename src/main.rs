use std::{net::Shutdown, process::exit};

use futures::StreamExt;
use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Flex, Layout, Margin, Position, Rect},
    style::{Color, Modifier, Style, Stylize},
    text::{Line, Span, Text},
    widgets::{
        Block, HighlightSpacing, List, ListItem, ListState, Paragraph, Scrollbar,
        ScrollbarOrientation, ScrollbarState, StatefulWidget, Widget,
    },
    DefaultTerminal, Frame,
};
use serde::{Deserialize, Serialize};
use tokio_util::io::StreamReader;

struct App {
    exit: bool,
    models_info: ModelInfo,
    selected_model: Model,
    ollama_api: OllamaApi,

    last_chat_area_height: usize,
    last_chat_area_width: usize,

    input: String,
    input_mode: InputMode,
    character_index: usize,

    chat_log: Vec<String>,
    chat_scroll_state: ScrollbarState,
    chat_scroll: usize,
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

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct Model {
    name: String,
    model: String,
    modified_at: String,
    size: u64,
    digest: String,
    details: ModelDetails,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
struct ModelDetails {
    format: String,
    family: String,
    families: Option<Vec<String>>,
    parameter_size: String,
    quantization_level: String,
}

#[derive(Serialize, Deserialize)]
struct ChatRequest {
    model: String,
    messages: Vec<MessageChunk>,
}

#[derive(Serialize, Deserialize, Debug)]
struct ChatResponse {
    model: String,
    created_at: String,
    message: MessageChunk,
    done: bool,
}

struct ChatResponseFinal {
    model: String,
    created_at: String,
    done: bool,
    total_duration: u64,
    load_duration: u64,
    prompt_eval_count: u64,
    prompt_eval_duration: u64,
    eval_count: u64,
    eval_duration: u64,
}

#[derive(Serialize, Deserialize, Debug)]
struct MessageChunk {
    role: String,
    content: String,
    images: Option<Vec<String>>,
}

struct OllamaApi {
    base_url: String,
    client: reqwest::Client,
}

impl Default for OllamaApi {
    fn default() -> Self {
        Self {
            base_url: "http://localhost:11434".to_string(),
            client: reqwest::Client::new(),
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
    async fn chat(&self, chat_request: ChatRequest) -> Result<String, reqwest::Error> {
        let response = self
            .client
            .post(format!("{}/api/chat", self.base_url))
            .body(serde_json::to_string(&chat_request).unwrap())
            .send()
            .await;

        // Return : -> Result<()>

        // let stream = res.bytes_stream().map_err(convert_err);
        // let mut lines_reader = StreamReader::new(stream).lines();

        // while let Ok(line) = lines_reader.next_line().await {
        //     if line.is_none() {
        //         break;
        //     }

        //     let ores: CompletionResponse = serde_json::from_str(&line.unwrap()).unwrap();
        //     tracing::debug!(body = ?ores, "Completion response");
        //     let mut msg = BackendResponse {
        //         author: Author::Model,
        //         text: ores.response,
        //         done: ores.done,
        //         context: None,
        //     };
        //     if ores.done && ores.context.is_some() {
        //         msg.context = Some(serde_json::to_string(&ores.context)?);
        //     }

        //     tx.send(Event::BackendPromptResponse(msg))?;
        // }

        match response {
            Ok(response) => {
                if response.status().is_success() {
                    let mut stream = response.bytes_stream();
                    while let Some(item) = stream.next().await {
                        let message = serde_json::from_slice::<ChatResponse>(&item.unwrap());
                        println!("Message: {:?}", message);
                    }
                }
            }
            Err(error) => {
                println!("error: {:?}", error);
            }
        }

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
            models_info: ModelInfo::default(),
            selected_model: Model::default(),
            ollama_api: OllamaApi::default(),
            last_chat_area_height: 0,
            last_chat_area_width: 0,
            input: String::new(),
            input_mode: InputMode::Normal,
            character_index: 0,
            chat_log: Vec::new(),
            chat_scroll_state: ScrollbarState::new(0).position(0),
            chat_scroll: 0,
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
                self.handle_key(key).await;
            }
        }

        Ok(())
    }

    async fn handle_key(&mut self, key: KeyEvent) {
        if key.kind != KeyEventKind::Press {
            return;
        }

        match self.input_mode {
            InputMode::Normal => match key.code {
                KeyCode::Esc => self.exit = true,
                KeyCode::Char('e') => {
                    if self.selected_model.name.is_empty() {
                        return;
                    }
                    self.input_mode = InputMode::Editing;
                }
                KeyCode::Down => self.models_info.selected_model.select_next(),
                KeyCode::Up => self.models_info.selected_model.select_previous(),
                KeyCode::Enter => self.select_model(),
                _ => {}
            },
            InputMode::Editing => match key.code {
                KeyCode::Enter => {
                    self.chat_message().await;

                    // TODO: analyze the code
                    if self.chat_log.len() > (self.last_chat_area_height - 2) {
                        self.chat_scroll = self.chat_log.len() - (self.last_chat_area_height - 2);
                    } else {
                        self.chat_scroll = 0;
                    }
                    self.chat_scroll_state.last();
                }
                KeyCode::Char(c) => self.update_input(c),
                KeyCode::Backspace => self.delete_input(),
                KeyCode::Left => self.move_cursor_left(),
                KeyCode::Right => self.move_cursor_right(),
                KeyCode::Down => {
                    // TODO: analyze the code
                    let mut clamp: usize = 0;
                    if self.chat_log.len() > self.last_chat_area_height - 2 {
                        clamp = self.chat_log.len() - (self.last_chat_area_height - 2) + 1;
                    }

                    self.chat_scroll = self
                        .chat_scroll
                        .saturating_add(1)
                        .clamp(0, clamp.saturating_sub(1));
                    self.chat_scroll_state.next();
                }
                KeyCode::Up => {
                    self.chat_scroll = self.chat_scroll.saturating_sub(1);
                    self.chat_scroll_state.prev();
                }
                KeyCode::Esc => self.input_mode = InputMode::Normal,
                _ => {}
            },
        }
    }

    fn set_chat_area_size(&mut self, area: Rect) {
        self.last_chat_area_height = area.height.into();
        self.last_chat_area_width = area.width.into();
    }

    fn select_model(&mut self) {
        if let Some(selected_model) = self.models_info.selected_model.selected() {
            self.selected_model = self.models_info.models.models[selected_model].clone();
        }
    }

    async fn chat_message(&mut self) {
        if self.input.is_empty() {
            return;
        }

        self.chat_log.push(self.input.clone());

        // TODO: ...
        self.chat(self.input.clone()).await;

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
            Constraint::Length(6),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(frame.area());

        // TODO: length should be dynamic based on the number of models
        let [header_area] = Layout::vertical([Constraint::Length(7)])
            .flex(Flex::Center)
            .areas(header_area);

        // TODO: separate input area
        let [list_area, chat_area] = Layout::vertical([
            Constraint::Length(self.models_info.models.models.len() as u16 + 2),
            Constraint::Fill(6),
        ])
        .areas(list_area);

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

        self.set_chat_area_size(chat_area);
    }

    fn render_header(&mut self, frame: &mut Frame, area: Rect) {
        let logo = r"       _ _                             _         _                
  ___ | | | __ _ _ __ ___   __ _      | |_ _   _(_)      _ __ ___ 
 / _ \| | |/ _` | '_ ` _ \ / _` |_____| __| | | | |_____| '__/ __|
| (_) | | | (_| | | | | | | (_| |_____| |_| |_| | |_____| |  \__ \
 \___/|_|_|\__,_|_| |_| |_|\__,_|      \__|\__,_|_|     |_|  |___/";

        let header = Paragraph::new(logo).centered();
        frame.render_widget(header, area);
    }

    fn render_model_list(&mut self, frame: &mut Frame, area: Rect) {
        let items: Vec<ListItem> = self
            .models_info
            .models
            .models
            .iter()
            .map(|model| {
                if model.name == self.selected_model.name {
                    ListItem::new(format!("✓ {}", model.name).fg(Color::Green))
                } else {
                    ListItem::new(format!("☐ {}", model.name))
                }
            })
            .collect();

        let list = List::new(items)
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        frame.render_stateful_widget(list, area, &mut self.models_info.selected_model);
    }

    // TODO: Scrollbar
    fn render_chat(&mut self, frame: &mut Frame, area: Rect) {
        let chat_log: Vec<Line> = self
            .chat_log
            .iter()
            .enumerate()
            .map(|(i, c)| Line::from(Span::raw(format!("{i}: {c}"))))
            .collect();

        let chat = Paragraph::new(chat_log)
            .block(Block::bordered().title("Chat"))
            .scroll((self.chat_scroll as u16, 0));
        frame.render_widget(chat, area);

        self.chat_scroll_state = self
            .chat_scroll_state
            .content_length(self.chat_log.len().saturating_sub(area.height as usize - 2));

        frame.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight)
                .begin_symbol(Some("↑"))
                .end_symbol(Some("↓")),
            area.inner(Margin {
                vertical: 1,
                horizontal: 0,
            }),
            &mut self.chat_scroll_state,
        );
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
        let normal_mode_text = "▲ ▼: model select, Enter: choose model, e: edit, Esc: quit";
        let editing_mode_text = "▲ ▼: chat scroll, Enter: send message, Esc: back to model select";

        let helper: Paragraph;
        match self.input_mode {
            InputMode::Normal => {
                helper = Paragraph::new(normal_mode_text).centered();
            }
            InputMode::Editing => {
                helper = Paragraph::new(editing_mode_text).centered();
            }
        }

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

    async fn chat(&mut self, prompt: String) {
        let chat_request = ChatRequest {
            model: self.selected_model.name.clone(),
            // TODO: add history
            messages: vec![MessageChunk {
                role: "user".to_string(),
                content: prompt,
                images: None,
            }],
        };

        let chat_response = self.ollama_api.chat(chat_request).await;
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
