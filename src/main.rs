use std::{net::Shutdown, process::exit};

use ratatui::{
    buffer::Buffer,
    crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind},
    layout::{Constraint, Flex, Layout, Rect},
    style::{Color, Modifier, Style, Stylize},
    widgets::{
        Block, HighlightSpacing, List, ListItem, ListState, Paragraph, StatefulWidget, Widget,
    },
    DefaultTerminal,
};
use serde::{Deserialize, Serialize};

struct App {
    exit: bool,
    models_info: ModelInfo,
    ollama_api: OllamaApi,
    input: String,
    input_mode: InputMode,
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
        }
    }
}

// App logic
impl App {
    async fn run(mut self, mut terminal: DefaultTerminal) -> std::io::Result<()> {
        self.load_models().await;

        let mut a = 0;
        while !self.exit {
            terminal.draw(|frame| frame.render_widget(&mut self, frame.area()))?;
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

        match key.code {
            KeyCode::Char('q') => self.exit = true,
            KeyCode::Down => self.models_info.selected_model.select_next(),
            KeyCode::Up => self.models_info.selected_model.select_previous(),
            KeyCode::Enter => self.select_model(),
            _ => {}
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
}

// UI
impl App {
    fn render_header(&mut self, area: Rect, buf: &mut Buffer) {
        Paragraph::new(
            r"
       _ _                             _         _                
  ___ | | | __ _ _ __ ___   __ _      | |_ _   _(_)      _ __ ___ 
 / _ \| | |/ _` | '_ ` _ \ / _` |_____| __| | | | |_____| '__/ __|
| (_) | | | (_| | | | | | | (_| |_____| |_| |_| | |_____| |  \__ \
 \___/|_|_|\__,_|_| |_| |_|\__,_|      \__|\__,_|_|     |_|  |___/",
        )
        .centered()
        .render(area, buf);
    }

    fn render_model_list(&mut self, area: Rect, buf: &mut Buffer) {
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

        StatefulWidget::render(list, area, buf, &mut self.models_info.selected_model);
    }

    fn render_text_input(&mut self, area: Rect, buf: &mut Buffer) {
        let input = Paragraph::new(self.input.as_str())
            .style(Style::default().fg(Color::Yellow))
            .block(Block::bordered().title("Input"));

        input.render(area, buf);
    }

    fn render_helper(&mut self, area: Rect, buf: &mut Buffer) {
        let text = "▲ ▼: select, Enter: choose, q: quit";
        Paragraph::new(text).centered().render(area, buf);
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

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [header_area, list_area, footer_area] = Layout::vertical([
            Constraint::Fill(1),
            Constraint::Fill(1),
            Constraint::Length(1),
        ])
        .areas(area);

        // TODO: length should be dynamic based on the number of models
        let [header_area] = Layout::vertical([Constraint::Length(7)])
            .flex(Flex::Center)
            .areas(header_area);

        // TODO: separate input area
        let [list_area, input_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Length(2)]).areas(list_area);

        // TODO: length should be dynamic based on the number of models
        let [list_area] = Layout::horizontal([Constraint::Length(15)])
            .flex(Flex::Center)
            .areas(list_area);
        let [list_area] = Layout::vertical([Constraint::Length(2)])
            .flex(Flex::Center)
            .areas(list_area);

        self.render_header(header_area, buf);
        self.render_model_list(list_area, buf);
        self.render_text_input(input_area, buf);
        self.render_helper(footer_area, buf);
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
