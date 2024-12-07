use std::time::{Duration, Instant};

use ratatui::{
    buffer::Buffer,
    layout::Rect,
    widgets::{Paragraph, Widget},
};

#[derive(Clone)]
pub struct TypedLine {
    /// Text to type
    pub text: String,
    /// Time to type the text in seconds
    pub time_to_type: Duration,
    /// Time to wait after typing the text in seconds
    pub time_to_wait: Duration,
}

pub struct TypeWriter {
    pub pages: Vec<TypedLine>,
    pub blinking_cursor_speed: Duration,
    pub started_at: Instant,
}

impl TypeWriter {
    pub fn new(
        started_at: Instant,
        blinking_cursor_speed: Duration,
        pages: Vec<TypedLine>,
    ) -> Self {
        Self {
            started_at,
            blinking_cursor_speed,
            pages,
        }
    }
}

impl TypeWriter {
    fn render_text(&self, elapsed: Duration) -> (String, bool) {
        let mut current_time = Duration::new(0, 0);
        let mut rendered_text = String::new();

        for line in &self.pages {
            match line.text.as_str() {
                "CLEAR" => {
                    rendered_text.clear();
                    current_time += line.time_to_type; // Add the time to type for CLEAR
                    if current_time + line.time_to_wait > elapsed {
                        break;
                    } else {
                        current_time += line.time_to_wait;
                    }
                }
                "BACK" => {
                    _ = rendered_text.pop();
                }
                _ => {
                    if current_time + line.time_to_type > elapsed {
                        let time_per_char = line.time_to_type / line.text.len() as u32;
                        let chars_to_render = ((elapsed - current_time).as_secs_f32()
                            / time_per_char.as_secs_f32())
                        .floor() as usize;

                        rendered_text.push_str(
                            &line
                                .text
                                .chars()
                                .into_iter()
                                .take(chars_to_render)
                                .collect::<String>(),
                        );
                        break;
                    } else {
                        rendered_text.push_str(&line.text);
                        current_time += line.time_to_type;
                    }

                    rendered_text.push('\n');
                    if current_time + line.time_to_wait > elapsed {
                        break;
                    } else {
                        current_time += line.time_to_wait;
                    }
                }
            }
        }

        let cursor_visible =
            (elapsed.as_millis() / self.blinking_cursor_speed.as_millis()) % 2 == 0;
        (rendered_text, cursor_visible)
    }
}

impl Widget for TypeWriter {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let elapsed = Instant::now() - self.started_at;
        let (mut text, cursor_visible) = self.render_text(elapsed);

        let line_count = text.lines().count();
        if line_count > area.height as usize {
            text = text
                .lines()
                .skip(line_count - area.height as usize)
                .collect::<Vec<&str>>()
                .join("\n");
        }

        let paragraph = Paragraph::new(text.clone());
        paragraph.render(area, buf);

        if cursor_visible {
            let cursor_pos = text.len();
            let mut x = 0;
            let mut y = 0;
            for (i, c) in text.chars().enumerate() {
                if i == cursor_pos {
                    break;
                }
                if c == '\n' {
                    x = 0;
                    y += 1;
                } else {
                    x += 1;
                    if x >= area.width as usize {
                        x = 0;
                        y += 1;
                    }
                }
            }
            if y < area.height as usize {
                buf.get_mut(area.left() + x as u16, area.top() + y as u16)
                    .set_symbol("_");
            }
        }
    }
}
