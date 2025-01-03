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
use ratatui::prelude::Layout;

static PATH_FAN: &str = "/proc/acpi/ibm/fan";

fn main() -> io::Result<()> {
    if !check_permissions() {
        update_permissions();
    }

    let mut terminal = ratatui::init();
    let app_result = App::new().run(&mut terminal);
    ratatui::restore();
    app_result
}

#[derive(Debug, Default, Clone)]
pub struct Input {
    name: String,
    temp: String,
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
    write_fan: &'static str,
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
            input.temp = temp;
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
            write_fan: "",
        }
    }

    /// runs the application's main loop until the user quits
    pub fn run(&mut self, terminal: &mut DefaultTerminal) -> io::Result<()> {
        while !self.exit {
            self.read_temperatures();
            self.read_fan();
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
        if !self.write_fan.is_empty() {
            write_to_fan(self.write_fan);
            self.write_fan = "";
        }
        Ok(())
    }

    fn handle_key_event(&mut self, key_event: KeyEvent) {
        match key_event.code {
            KeyCode::Char('q') => self.exit(),
            KeyCode::Char('f') => self.write_fan = "level full-speed",
            KeyCode::Char('a') => self.write_fan = "level auto",
            KeyCode::Char('0') => self.write_fan = "level 0",
            KeyCode::Char('1') => self.write_fan = "level 1",
            KeyCode::Char('2') => self.write_fan = "level 2",
            KeyCode::Char('3') => self.write_fan = "level 3",
            KeyCode::Char('4') => self.write_fan = "level 4",
            KeyCode::Char('5') => self.write_fan = "level 5",
            KeyCode::Char('6') => self.write_fan = "level 6",
            KeyCode::Char('7') => self.write_fan = "level 7",
            _ => {}
        }
    }

    fn exit(&mut self) {
        self.exit = true;
    }

    fn read_fan(&mut self) {
        let file = File::open(PATH_FAN).unwrap();
        let mut buffer = [0; 64];
        let mut handle = file.take(63);
        handle.read(&mut buffer).unwrap();
        let lines: Vec<&str> = std::str::from_utf8(&buffer).unwrap().lines().collect();
        if lines.len() < 3 {
            return;
        }
        self.lines = lines
            .iter()
            .take(3)
            .map(|line| {
                let split: Vec<_> = line.split_whitespace().collect();
                format!("{} {:>8}", split[0], split[1])
            })
            .collect();
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

fn update_permissions() {
    let username = whoami::username();
    let _output = Command::new("pkexec")
        .arg("chown")
        .arg(username)
        .arg(PATH_FAN)
        .output()
        .expect("failed to run chown command");
}

fn check_permissions() -> bool {
    let Ok(f) = File::create(PATH_FAN) else {
        return false;
    };
    let metadata = f.metadata().unwrap();
    return !metadata.permissions().readonly();
}

fn write_to_fan(text: &str) {
    std::fs::write(PATH_FAN, text).expect("Unable to write file");
}

fn lines_to_text(lines: &Vec<String>) -> Text {
    return Text::from(
        lines
            .iter()
            .map(|line| Line::from(line.clone()))
            .collect::<Vec<_>>(),
    );
}

fn pad_right(input: &str, width: usize) -> String {
    let mut result = input.to_string();
    for _i in result.len()..width {
        result.push(' ');
    }
    return result;
}

fn adapters_to_table(adapters: &Vec<Adapter>) -> Paragraph {
    let mut rows = Vec::new();
    let mut max_widths: [usize; 3] = [7, 5, 9];

    for adapter in adapters {
        for input in &adapter.inputs {
            max_widths[0] = std::cmp::max(max_widths[0], adapter.name.len());
            max_widths[1] = std::cmp::max(max_widths[1], input.name.len());
            max_widths[2] = std::cmp::max(max_widths[2], input.temp.len());
        }
    }

    let line_width = max_widths[0] + max_widths[1] + max_widths[2];

    let header = pad_right("  Adapter", max_widths[0] + 5)
        + pad_right("Input", max_widths[1] + 3).as_str()
        + pad_right("Temp. °C", max_widths[2] + 2).as_str();
    rows.push(Line::from(header));

    let mut head_border = String::with_capacity(line_width + 10);
    head_border += "┏";
    head_border += "━".repeat(max_widths[0] + 2).as_str();
    head_border += "┳";
    head_border += "━".repeat(max_widths[1] + 2).as_str();
    head_border += "┳";
    head_border += "━".repeat(max_widths[2] + 2).as_str();
    head_border += "┓";
    rows.push(Line::from(head_border));

    for adapter in adapters {
        for input in adapter.inputs.clone() {
            let mut row = String::with_capacity(line_width + 10);
            row += "┃ ";
            row += pad_right(adapter.name.as_str(), max_widths[0]).as_str();
            row += " ┃ ";
            row += pad_right(input.name.as_str(), max_widths[1]).as_str();
            row += " ┃ ";
            row += pad_right(input.temp.as_str(), max_widths[2]).as_str();
            row += " ┃";
            rows.push(Line::from(row));
        }
    }

    let mut bottom_border = String::with_capacity(line_width + 10);
    bottom_border += "┗";
    bottom_border += "━".repeat(max_widths[0] + 2).as_str();
    bottom_border += "┻";
    bottom_border += "━".repeat(max_widths[1] + 2).as_str();
    bottom_border += "┻";
    bottom_border += "━".repeat(max_widths[2] + 2).as_str();
    bottom_border += "┛";
    rows.push(Line::from(bottom_border));

    return Paragraph::new(rows);
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

        let areas = Layout::vertical([Constraint::Max(5), Constraint::Min(0)])
            .split(area)
            .to_vec();

        Paragraph::new(lines_to_text(&self.lines))
            .centered()
            .block(block_up)
            .render(areas[0], buf);

        adapters_to_table(&self.adapters)
            .centered()
            .block(block_down)
            .render(areas[1], buf);
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
        assert_eq!(adapters[0].inputs[0].temp, "50.0");

        assert_eq!(adapters[1].name, "amdgpu-pci-0700");
        assert_eq!(adapters[1].inputs[0].name, "edge");
        assert_eq!(adapters[1].inputs[0].temp, "48.0");

        assert_eq!(adapters[2].name, "iwlwifi_1-virtual-0");
        assert_eq!(adapters[2].inputs[0].name, "temp1");
        assert_eq!(adapters[2].inputs[0].temp, "43.0");

        assert_eq!(adapters[3].name, "k10temp-pci-00c3");
        assert_eq!(adapters[3].inputs[0].name, "Tctl");
        assert_eq!(adapters[3].inputs[0].temp, "49.875");

        assert_eq!(adapters[4].name, "nvme-pci-0100");
        assert_eq!(adapters[4].inputs[0].name, "Composite");
        assert_eq!(adapters[4].inputs[0].temp, "37.85");
        assert_eq!(adapters[4].inputs[1].name, "Sensor 1");
        assert_eq!(adapters[4].inputs[1].temp, "37.85");
        assert_eq!(adapters[4].inputs[2].name, "Sensor 2");
        assert_eq!(adapters[4].inputs[2].temp, "38.85");

        assert_eq!(adapters[5].name, "nvme-pci-0500");
        assert_eq!(adapters[5].inputs[0].name, "Composite");
        assert_eq!(adapters[5].inputs[0].temp, "41.85");

        assert_eq!(adapters[6].name, "thinkpad-isa-0000");
        assert_eq!(adapters[6].inputs[0].name, "CPU");
        assert_eq!(adapters[6].inputs[0].temp, "50.0");
    }
}
