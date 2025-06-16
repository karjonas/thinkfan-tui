use std::time::Duration;

use std::fs::File;
use std::io;
use std::io::Read;
use std::process::Command;

use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind};
use ratatui::{
    buffer::Buffer,
    layout::Rect,
    style::Stylize,
    symbols::border,
    text::{Line, Text},
    widgets::{Block, Paragraph, Widget},
    DefaultTerminal, Frame,
};

use ratatui::prelude::Constraint;
use ratatui::prelude::Direction;
use ratatui::prelude::Layout;

use ratatui::prelude::Alignment;
use ratatui::style::Color;
use ratatui::style::Style;
use ratatui::text::Span;

static PATH_FAN: &str = "/proc/acpi/ibm/fan";

// Light (foreground) filled bar colors
static GREEN_LIGHT: Color = Color::Rgb(165, 183, 0); // #a5b700
static YELLOW_LIGHT: Color = Color::Rgb(227, 168, 43); // #e3a82b
static RED_LIGHT: Color = Color::Rgb(204, 31, 26); // #cc1f1a

// Dark (background) bar colors
static GREEN_DARK: Color = Color::Rgb(68, 68, 37); // #444425
static YELLOW_DARK: Color = Color::Rgb(94, 78, 40); // #5e4e28
static RED_DARK: Color = Color::Rgb(76, 32, 32); // #4c2020

fn main() -> io::Result<()> {
    if !check_permissions() && !update_permissions() {
        println!("Error: could not update permissions");
        return Ok(());
    }

    let mut terminal = ratatui::init();
    let app_result = App::new().run(&mut terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug, Default, Clone)]
pub struct Input {
    name: String,
    temp: f64,
}

#[derive(Debug, Default, Clone)]
pub struct Adapter {
    name: String,
    inputs: Vec<Input>,
}

#[derive(Debug)]
pub struct App {
    exit: bool,
    lines: Vec<String>,
    adapters: Vec<Adapter>,
    fan_command: &'static str,
    current_error: String,
}

fn parse_adapters(json_str: &str) -> Vec<Adapter> {
    let json: serde_json::Value =
        serde_json::from_str(json_str).expect("JSON was not well-formatted");
    let json_obj = json.as_object().unwrap();
    let mut adapters = Vec::new();

    for adapter_name in json_obj.keys() {
        let mut curr_inputs: Vec<(String, String)> = Vec::new();
        let adapter = json_obj[adapter_name].as_object().unwrap();

        for input_key in adapter.keys() {
            let input = adapter[input_key].clone();
            if !input.is_object() {
                continue;
            }

            let input_obj = input.as_object().unwrap();
            for temp_key in input_obj.keys() {
                if !temp_key.contains("temp") || !temp_key.contains("input") {
                    continue;
                }

                if input_obj[temp_key] == 0.0 {
                    continue;
                }

                curr_inputs.push((input_key.clone(), input_obj[temp_key].to_string()));
            }
        }

        if curr_inputs.is_empty() {
            continue;
        }

        let mut adapter = Adapter::default();
        adapter.name = adapter_name.clone();

        for (name, temp) in curr_inputs {
            let mut input = Input::default();
            input.name = name;
            input.temp = temp.trim().parse::<f64>().unwrap_or(-99.0);
            adapter.inputs.push(input);
        }
        adapters.push(adapter);
    }

    return adapters;
}

impl App {
    pub fn new() -> Self {
        Self {
            exit: false,
            lines: Vec::new(),
            adapters: Vec::new(),
            fan_command: "",
            current_error: String::new(),
        }
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            self.read_temperatures();
            self.read_fan();
            // Add error if present
            if !self.current_error.is_empty() {
                self.lines.push(self.current_error.clone());
            }
            terminal.draw(|frame| self.draw(frame))?;
            self.handle_events()?;
        }
        Ok(())
    }

    fn draw(&self, frame: &mut Frame) {
        frame.render_widget(self, frame.area());
    }

    /// updates the application's state based on user input
    fn handle_events(&mut self) -> io::Result<()> {
        let timeout = Duration::from_secs_f32(1.0);
        if event::poll(timeout)? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    self.handle_key_event(key)
                }
            }
        };
        while event::poll(Duration::from_secs_f32(0.0))? {
            if let Event::Key(key) = event::read()? {
                if key.kind == KeyEventKind::Press {
                    self.handle_key_event(key)
                }
            }
        }
        self.write_command_to_fan();
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('f') => self.fan_command = "level full-speed",
            KeyCode::Char('a') => self.fan_command = "level auto",
            KeyCode::Char('0') => self.fan_command = "level 0",
            KeyCode::Char('1') => self.fan_command = "level 1",
            KeyCode::Char('2') => self.fan_command = "level 2",
            KeyCode::Char('3') => self.fan_command = "level 3",
            KeyCode::Char('4') => self.fan_command = "level 4",
            KeyCode::Char('5') => self.fan_command = "level 5",
            KeyCode::Char('6') => self.fan_command = "level 6",
            KeyCode::Char('7') => self.fan_command = "level 7",
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn read_fan(&mut self) {
        self.lines.clear();
        let file = match File::open(PATH_FAN) {
            Ok(f) => f,
            Err(_) => {
                self.current_error = "Failed to open file: ".to_string() + PATH_FAN;
                return;
            }
        };

        let mut buffer = [0; 64];
        let mut handle = file.take(63);

        if let Err(_) = handle.read(&mut buffer) {
            self.current_error = "Failed to read from file: ".to_string() + PATH_FAN;
            return;
        }

        let lines: Vec<&str> = match std::str::from_utf8(&buffer) {
            Ok(s) => s.lines().collect(),
            Err(_) => {
                self.current_error = "Invalid UTF-8 in file: ".to_string() + PATH_FAN;
                return;
            }
        };

        if lines.len() < 3 {
            self.current_error = "Unexpected number of lines in file: ".to_string() + PATH_FAN;
            return;
        }

        self.lines = lines
            .iter()
            .take(3)
            .map(|line| {
                let split: Vec<_> = line.split_whitespace().collect();
                format!("{} {:>10}", split[0], split[1])
            })
            .collect();
    }

    fn write_command_to_fan(&mut self) {
        if self.fan_command.is_empty() {
            return;
        }

        match std::fs::write(PATH_FAN, self.fan_command) {
            Ok(_) => self.current_error = String::new(),
            Err(_) => {
                self.current_error =
                    "Failed to write command '".to_string() + self.fan_command + "' to " + PATH_FAN
            }
        }

        self.fan_command = "";
    }

    fn read_temperatures(&mut self) {
        let output = Command::new("sensors")
            .arg("-j")
            .output()
            .expect("failed to run sensors command");
        let json_str = std::str::from_utf8(&output.stdout).unwrap();
        self.adapters = parse_adapters(json_str);
    }
}

fn update_permissions() -> bool {
    let username = whoami::username();
    let output = Command::new("pkexec")
        .arg("chown")
        .arg(username)
        .arg(PATH_FAN)
        .output()
        .expect("failed to run chown command");
    return output.status.success() && output.status.code().unwrap_or(-1) == 0;
}

fn check_permissions() -> bool {
    let Ok(f) = File::create(PATH_FAN) else {
        return false;
    };
    let metadata = f.metadata().unwrap();
    return !metadata.permissions().readonly();
}

fn lines_to_text(lines: &Vec<String>) -> Text {
    return Text::from(
        lines
            .iter()
            .map(|line| Line::from(line.clone()))
            .collect::<Vec<_>>(),
    );
}

impl Widget for &App {
    fn render(self, area: Rect, buf: &mut Buffer) {
        let title_up = Line::from(" Fan Info ".bold());
        let title_down = Line::from(" Temperatures ".bold());
        let instructions = Line::from(vec![
            " Level ".into(),
            "<0-7>".bold(),
            ", Auto ".into(),
            "<A>".bold(),
            ", Full ".into(),
            "<F>".bold(),
            ", Quit ".into(),
            "<Q> ".bold(),
        ]);

        let block_up = Block::bordered()
            .title(title_up.centered())
            .border_set(border::THICK);

        let block_down = Block::bordered()
            .title(title_down.centered())
            .title_bottom(instructions.centered())
            .border_set(border::THICK);

        let areas = Layout::vertical([
            Constraint::Max(2 + self.lines.len() as u16),
            Constraint::Min(0),
        ])
        .split(area);

        // Top info block
        Paragraph::new(lines_to_text(&self.lines))
            .centered()
            .block(block_up)
            .render(areas[0], buf);

        // Layout for adapter content
        let inner_area = block_down.inner(areas[1]);
        let padded_area = Rect {
            x: inner_area.x + 1,
            y: inner_area.y,
            width: inner_area.width.saturating_sub(2),
            height: inner_area.height,
        };

        // Check if we have enough space to render bars
        let total_inputs: usize = self.adapters.iter().map(|a| a.inputs.len()).sum();
        let min_required_height = total_inputs * 2;

        let render_bars = padded_area.height as usize >= min_required_height;

        let mut constraints = Vec::with_capacity(total_inputs * 2);
        for _ in 0..total_inputs {
            constraints.push(Constraint::Length(1)); // label
            if render_bars {
                constraints.push(Constraint::Length(1)); // bar
            }
        }

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(constraints)
            .split(padded_area);

        let mut i = 0;
        for adapter in &self.adapters {
            for input in &adapter.inputs {
                let label = format!("{} | {}", adapter.name, input.name);
                let fill_ratio = (input.temp / 100.0).clamp(0.0, 1.0);
                let dot_color = match fill_ratio {
                    t if t < 0.45 => GREEN_LIGHT,
                    t if t < 0.75 => YELLOW_LIGHT,
                    _ => RED_LIGHT,
                };

                let temp_spans = Line::from(vec![
                    Span::raw(format!("{}°C ", input.temp as i8)),
                    Span::styled("▊", Style::default().fg(dot_color)),
                ]);

                // Row: label + temperature
                let row_chunks = Layout::horizontal([Constraint::Min(0), Constraint::Length(6)])
                    .split(chunks[i]);

                Paragraph::new(Line::from(Span::raw(label))).render(row_chunks[0], buf);
                Paragraph::new(temp_spans)
                    .alignment(Alignment::Right)
                    .render(row_chunks[1], buf);

                i += 1;

                // Conditionally render the bar
                if render_bars {
                    let width = chunks[i].width as usize;
                    let filled = (fill_ratio * width as f64).round() as usize;

                    let spans: Vec<Span> = (0..width)
                        .map(|idx| {
                            let ratio = idx as f64 / width as f64;
                            let color = if idx < filled {
                                if ratio < 0.45 {
                                    GREEN_LIGHT
                                } else if ratio < 0.75 {
                                    YELLOW_LIGHT
                                } else {
                                    RED_LIGHT
                                }
                            } else {
                                if ratio < 0.45 {
                                    GREEN_DARK
                                } else if ratio < 0.75 {
                                    YELLOW_DARK
                                } else {
                                    RED_DARK
                                }
                            };

                            Span::styled("▀", Style::default().fg(color))
                        })
                        .collect();

                    Paragraph::new(Line::from(spans)).render(chunks[i], buf);
                    i += 1;
                }
            }
        }

        // Final block border
        block_down.render(areas[1], buf);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sensors_t14s() {
        let json_str: String = std::fs::read_to_string("testdata/sensors-t14s-amd-gen1").unwrap();
        let adapters = parse_adapters(json_str.as_str());
        assert_eq!(adapters.len(), 7);

        assert_eq!(adapters[0].name, "acpitz-acpi-0");
        assert_eq!(adapters[0].inputs[0].name, "temp1");
        assert_eq!(adapters[0].inputs[0].temp, 50.0);

        assert_eq!(adapters[1].name, "amdgpu-pci-0700");
        assert_eq!(adapters[1].inputs[0].name, "edge");
        assert_eq!(adapters[1].inputs[0].temp, 48.0);

        assert_eq!(adapters[2].name, "iwlwifi_1-virtual-0");
        assert_eq!(adapters[2].inputs[0].name, "temp1");
        assert_eq!(adapters[2].inputs[0].temp, 43.0);

        assert_eq!(adapters[3].name, "k10temp-pci-00c3");
        assert_eq!(adapters[3].inputs[0].name, "Tctl");
        assert_eq!(adapters[3].inputs[0].temp, 49.875);

        assert_eq!(adapters[4].name, "nvme-pci-0100");
        assert_eq!(adapters[4].inputs[0].name, "Composite");
        assert_eq!(adapters[4].inputs[0].temp, 37.85);
        assert_eq!(adapters[4].inputs[1].name, "Sensor 1");
        assert_eq!(adapters[4].inputs[1].temp, 37.85);
        assert_eq!(adapters[4].inputs[2].name, "Sensor 2");
        assert_eq!(adapters[4].inputs[2].temp, 38.85);

        assert_eq!(adapters[5].name, "nvme-pci-0500");
        assert_eq!(adapters[5].inputs[0].name, "Composite");
        assert_eq!(adapters[5].inputs[0].temp, 41.85);

        assert_eq!(adapters[6].name, "thinkpad-isa-0000");
        assert_eq!(adapters[6].inputs[0].name, "CPU");
        assert_eq!(adapters[6].inputs[0].temp, 50.0);
    }

    #[test]
    fn sensors_t490() {
        let json_str: String = std::fs::read_to_string("testdata/sensors-t490").unwrap();
        let adapters = parse_adapters(json_str.as_str());
        assert_eq!(adapters.len(), 6);

        assert_eq!(adapters[0].name, "acpitz-acpi-0");
        assert_eq!(adapters[0].inputs[0].name, "temp1");
        assert_eq!(adapters[0].inputs[0].temp, 46.0);

        assert_eq!(adapters[1].name, "coretemp-isa-0000");
        assert_eq!(adapters[1].inputs[0].name, "Core 0");
        assert_eq!(adapters[1].inputs[0].temp, 47.0);
        assert_eq!(adapters[1].inputs[1].name, "Core 1");
        assert_eq!(adapters[1].inputs[1].temp, 49.0);
        assert_eq!(adapters[1].inputs[2].name, "Core 2");
        assert_eq!(adapters[1].inputs[2].temp, 51.0);
        assert_eq!(adapters[1].inputs[3].name, "Core 3");
        assert_eq!(adapters[1].inputs[3].temp, 49.0);
        assert_eq!(adapters[1].inputs[4].name, "Package id 0");
        assert_eq!(adapters[1].inputs[4].temp, 51.0);

        assert_eq!(adapters[2].name, "iwlwifi_1-virtual-0");
        assert_eq!(adapters[2].inputs[0].name, "temp1");
        assert_eq!(adapters[2].inputs[0].temp, 54.0);

        assert_eq!(adapters[3].name, "nvme-pci-3d00");
        assert_eq!(adapters[3].inputs[0].name, "Composite");
        assert_eq!(adapters[3].inputs[0].temp, 43.85);
        assert_eq!(adapters[3].inputs[1].name, "Sensor 1");
        assert_eq!(adapters[3].inputs[1].temp, 43.85);
        assert_eq!(adapters[3].inputs[2].name, "Sensor 2");
        assert_eq!(adapters[3].inputs[2].temp, 42.85);

        assert_eq!(adapters[4].name, "pch_cannonlake-virtual-0");
        assert_eq!(adapters[4].inputs[0].name, "temp1");
        assert_eq!(adapters[4].inputs[0].temp, 43.0);

        assert_eq!(adapters[5].name, "thinkpad-isa-0000");
        assert_eq!(adapters[5].inputs[0].name, "CPU");
        assert_eq!(adapters[5].inputs[0].temp, 46.0);
        assert_eq!(adapters[5].inputs[1].name, "temp5");
        assert_eq!(adapters[5].inputs[1].temp, 34.0);
    }
}
