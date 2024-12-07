mod widgets;

use widgets::typewriter::{TypeWriter, TypedLine};

use std::{
    io::{self, Cursor},
    process,
    time::{Duration, Instant},
};

use crossterm::event::{self, poll, Event, KeyCode, KeyEvent, KeyEventKind};
use image::{ImageReader, Rgb};
use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Style},
    widgets::{Block, Paragraph},
    DefaultTerminal, Frame,
};
use ratatui_image::{picker::Picker, protocol::StatefulProtocol, StatefulImage};

fn generate_scroll_line() -> String {
    let mut text = String::new();
    for _ in 0..12 {
        let random = rand::random::<u8>();
        let character = match random % 2 {
            0 => ' ',
            1 => '-',
            _ => unreachable!(),
        };
        text.push(character);
    }
    text
}

#[macro_export]
macro_rules! load_typed_lines_from_csv {
    ($file_path:expr) => {{
        use csv::ReaderBuilder;
        use std::time::Duration;

        let csv_content = include_str!($file_path);
        let mut rdr = ReaderBuilder::new()
            .has_headers(false)
            .from_reader(csv_content.as_bytes());
        let mut typed_lines = Vec::new();

        for result in rdr.records() {
            let record = result?;
            let text = record.get(0).ok_or("Missing text").unwrap().to_string();
            let time_to_type = record
                .get(1)
                .ok_or("Missing time_to_type")
                .unwrap()
                .parse::<u64>()
                .unwrap();
            let time_to_wait = record
                .get(2)
                .ok_or("Missing time_to_wait")
                .unwrap()
                .parse::<u64>()
                .unwrap();

            typed_lines.push(TypedLine {
                text,
                time_to_type: Duration::from_millis(time_to_type),
                time_to_wait: Duration::from_millis(time_to_wait),
            });
        }

        typed_lines
    }};
}

use rodio::{Decoder, OutputStream, Sink};
fn main() -> io::Result<()> {
    let (_stream, stream_handle) = OutputStream::try_default().unwrap();
    let audio_data = include_bytes!("../assets/audio.flac");
    let cursor = std::io::Cursor::new(audio_data);
    let sink = Sink::try_new(&stream_handle).unwrap();
    sink.pause();
    let source = Decoder::new(cursor).unwrap();
    sink.append(source);
    sink.set_volume(0.2);

    let mut terminal = ratatui::init();

    let mut picker = match Picker::from_query_stdio() {
        Ok(picker) => picker,
        Err(e) => {
            println!(
                "Error determining graphics capabilities of the terminal: {e} (Does your terminal support Sixel?)"
            );
            process::exit(1);
        }
    };

    let aperture_img_bytes = include_bytes!("../assets/aperture.png");

    let aperture_img = match ImageReader::new(Cursor::new(aperture_img_bytes))
        .with_guessed_format()?
        .decode()
    {
        Ok(img) => img,
        Err(e) => {
            panic!("Error decoding image: {e} (Are you using Windows?)");
        }
    };

    let tui_aperture_img = picker.new_resize_protocol(aperture_img);

    let mut upscroll_text = String::new();
    for _ in 0..15 {
        upscroll_text.push_str(&generate_scroll_line());
        upscroll_text.push('\n');
    }

    let mut downscroll_text = String::new();
    for _ in 0..15 {
        downscroll_text.push_str(&generate_scroll_line());
        downscroll_text.push('\n');
    }

    let lyrics: Vec<TypedLine> = load_typed_lines_from_csv!("../assets/lyrics.csv");
    let credits: Vec<TypedLine> = load_typed_lines_from_csv!("../assets/credits.csv");

    let app_result = App {
        exit: false,
        image: tui_aperture_img,
        upscroll_text,
        downscroll_text,
        started_at: Instant::now(),
        sink,
        lyrics,
        credits,
    }
    .run(&mut terminal);

    ratatui::restore();

    app_result
}
pub struct App {
    exit: bool,
    image: StatefulProtocol,
    upscroll_text: String,
    downscroll_text: String,
    started_at: Instant,
    sink: Sink,
    lyrics: Vec<TypedLine>,
    credits: Vec<TypedLine>,
}

impl App {
    /// Runs the application until the user quits by pressing 'q' or 'c'.
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        self.sink.play();
        while !self.exit && !self.sink.empty() {
            terminal.draw(|frame| self.draw(frame))?;
            self.scroll_scrolltext();
            self.handle_events()?;
        }
        Ok(())
    }

    /// Scrolls the scrolltext by one line.
    fn scroll_scrolltext(&mut self) {
        let mut upscroll_lines = self.upscroll_text.lines().collect::<Vec<&str>>();
        upscroll_lines.remove(0);
        let upscroll_line = generate_scroll_line();
        upscroll_lines.push(&upscroll_line);
        self.upscroll_text = upscroll_lines.join("\n");

        let mut downscroll_lines = self.downscroll_text.lines().collect::<Vec<&str>>();
        downscroll_lines.pop();
        let downscroll_line = generate_scroll_line();
        downscroll_lines.insert(0, &downscroll_line);
        self.downscroll_text = downscroll_lines.join("\n");
    }

    fn draw(&mut self, frame: &mut Frame) {
        const BG_COLOR: Color = Color::Rgb(43, 20, 0);
        const FG_COLOR: Color = Color::Rgb(248, 180, 0);

        const STYLE: Style = Style::new().bg(BG_COLOR).fg(FG_COLOR);

        let background = Block::default().style(STYLE);

        frame.render_widget(background, frame.area());

        let border = Block::bordered();

        let border_layout = Layout::default()
            .margin(1)
            .constraints(vec![Constraint::Fill(1)])
            .split(frame.area());
        frame.render_widget(border.clone(), border_layout[0]);

        let layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Fill(3), Constraint::Fill(1)])
            .split(frame.area());

        let lyrics_layout = Layout::default()
            .margin(3)
            .constraints(vec![Constraint::Fill(1)])
            .split(layout[0]);

        let right_layout = Layout::default()
            .direction(Direction::Vertical)
            .constraints(vec![Constraint::Length(6), Constraint::Fill(1)])
            .split(layout[1]);

        let credits_layout = Layout::default()
            .constraints(vec![Constraint::Fill(1)])
            .margin(3)
            .split(right_layout[1]);

        let right_header_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![
                Constraint::Length(28),
                Constraint::Length(6),
                Constraint::Length(11),
                Constraint::Fill(1),
            ])
            .split(right_layout[0]);

        let image_layout = Layout::default()
            .margin(1)
            .constraints(vec![Constraint::Fill(1)])
            .split(right_header_layout[2]);

        let header_scroll_layout = Layout::default()
            .direction(Direction::Horizontal)
            .constraints(vec![Constraint::Percentage(50), Constraint::Percentage(50)])
            .split(right_header_layout[0]);

        let upscroll = Paragraph::new(self.upscroll_text.clone()).block(Block::bordered());
        let downscroll = Paragraph::new(self.downscroll_text.clone()).block(Block::bordered());

        frame.render_widget(upscroll, header_scroll_layout[1]);
        frame.render_widget(downscroll, header_scroll_layout[0]);

        let stateful_image = StatefulImage::new(Some(Rgb([43, 20, 0])));
        let version = Paragraph::new("2.67\n1002\n45.6")
            .block(Block::bordered())
            .style(Style::new().fg(Color::Rgb(248, 180, 0)));

        frame.render_widget(version, right_header_layout[1]);

        frame.render_widget(Paragraph::new("               "), image_layout[0]);
        frame.render_stateful_widget(stateful_image, image_layout[0], &mut self.image);
        frame.render_widget(border, right_header_layout[2]);

        let lyrics = TypeWriter::new(
            self.started_at,
            Duration::from_millis(500),
            self.lyrics.clone(),
        );
        let credits = TypeWriter::new(
            self.started_at,
            Duration::from_millis(500),
            self.credits.clone(),
        );

        frame.render_widget(lyrics, lyrics_layout[0]);
        frame.render_widget(credits, credits_layout[0]);
    }

    fn handle_events(&mut self) -> io::Result<()> {
        if !poll(Duration::from_millis(50))? {
            return Ok(());
        }
        match event::read()? {
            Event::Key(key_event) if key_event.kind == KeyEventKind::Press => {
                self.handle_key_event(key_event)
            }
            _ => {}
        };
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('c') => self.exit(),
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }
}
