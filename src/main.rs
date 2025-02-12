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

struct App {
    exit: bool,
    model_list: ModelList,
}

struct ModelList {
    models: Vec<Model>,
    selected_model: ListState,
}

struct Model {
    name: String,
}

impl Default for App {
    fn default() -> Self {
        Self {
            exit: false,
            // TODO: load models from api
            model_list: ModelList {
                models: vec![
                    Model {
                        name: "Model 1".to_string(),
                    },
                    Model {
                        name: "Model 2".to_string(),
                    },
                ],
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
            KeyCode::Down => self.model_list.selected_model.select_next(),
            KeyCode::Up => self.model_list.selected_model.select_previous(),
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
        .bold()
        .centered()
        .render(area, buf);
    }

    fn render_model_list(&mut self, area: Rect, buf: &mut Buffer) {
        let items: Vec<ListItem> = self
            .model_list
            .models
            .iter()
            .map(|model| ListItem::new(format!("{}", model.name)))
            .collect();

        let list = List::new(items)
            .highlight_symbol("> ")
            .highlight_spacing(HighlightSpacing::Always);

        StatefulWidget::render(list, area, buf, &mut self.model_list.selected_model);
    }
}

impl Widget for &mut App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let [header_area, list_area] =
            Layout::vertical([Constraint::Length(8), Constraint::Fill(1)]).areas(area);

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

fn main() -> std::io::Result<()> {
    let terminal = ratatui::init();
    let result = App::default().run(terminal);
    ratatui::restore();
    result
}
