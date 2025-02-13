use std::process::exit;

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

impl Default for App {
    fn default() -> Self {
        Self {
            exit: false,
            // TODO: load models from api
            models_info: ModelInfo {
                models: ModelList {
                    models: vec![Model {
                        name: "Model 1".to_string(),
                        model: "model1".to_string(),
                        modified_at: "2021-01-01".to_string(),
                        size: 1000,
                        digest: "digest1".to_string(),
                        details: ModelDetails {
                            format: "gguf".to_string(),
                            family: "llama".to_string(),
                            families: None,
                            parameter_size: "7B".to_string(),
                            quantization_level: "Q4_0".to_string(),
                        },
                    }],
                },
                selected_model: ListState::default(),
            },
        }
    }
}

impl App {
    fn run(mut self, mut terminal: DefaultTerminal) -> std::io::Result<()> {
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
            KeyCode::Enter => {}
            _ => {}
        }
    }

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
            .map(|model| ListItem::new(format!("{}", model.name)))
            .collect();

        let list = List::new(items)
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut self.models_info.selected_model);
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [header_area, list_area] =
            Layout::vertical([Constraint::Fill(1), Constraint::Fill(1)]).areas(area);

        // TODO: length should be dynamic based on the number of models
        let [header_area] = Layout::vertical([Constraint::Length(7)])
            .flex(Flex::Center)
            .areas(header_area);

        // TODO: length should be dynamic based on the number of models
        let [list_area] = Layout::horizontal([Constraint::Length(15)])
            .flex(Flex::Center)
            .areas(list_area);
        let [list_area] = Layout::vertical([Constraint::Length(2)])
            .flex(Flex::Center)
            .areas(list_area);

        self.render_header(header_area, buf);
        self.render_model_list(list_area, buf);
    }
}

#[tokio::main]
async fn main() -> std::io::Result<()> {
    let res = reqwest::get("http://localhost:11434/api/tags").await;
    match res {
        Ok(res) => {
            let body = res.bytes().await.unwrap();
            let models = serde_json::from_slice::<ModelList>(&body)?;
            println!("{:?}", models);
        }
        Err(e) => {
            println!("Error: {}", e);
        }
    }

    let terminal = ratatui::init();
    let result = App::default().run(terminal);
    ratatui::restore();
    result
}
