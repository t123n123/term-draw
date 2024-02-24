use std::{
    collections::HashMap,
    io::{self, Stdout, BufWriter, Write},
    time::Duration, cmp::{min, max}, fs::File,
};

use anyhow::{Context, Result};
use crossterm::{
    event::{self, Event, KeyCode},
    execute,
    terminal::{disable_raw_mode, enable_raw_mode, EnterAlternateScreen, LeaveAlternateScreen},
};
use ratatui::{prelude::*, widgets::*};

#[derive(PartialEq)]
enum InputState {
    INSERT,
    BOX,
    ARROW,
}

struct GlobalState {
    color: bool,
    should_quit: bool,
    start_pos: (u16, u16),
    prev_pos: (u16, u16),
    current_pos: (u16, u16),
    diagram: HashMap<(u16, u16), char>,
    preview: HashMap<(u16, u16), char>,
    window_size: (u16, u16),
    input_state: InputState,
}

impl Default for GlobalState {
    fn default() -> GlobalState {
        GlobalState {
            color: false,
            should_quit: false,
            start_pos: (0, 0),
            prev_pos: (0, 0),
            current_pos: (0, 0),
            diagram: HashMap::new(),
            preview: HashMap::new(),
            window_size: (0, 0),
            input_state: InputState::INSERT,
        }
    }
}

fn main() -> Result<()> {
    let mut terminal = setup_terminal().context("setup failed")?;
    let mut global_state = GlobalState::default();
    run(&mut terminal, &mut global_state).context("app loop failed")?;
    restore_terminal(&mut terminal).context("restore terminal failed")?;
    Ok(())
}

fn setup_terminal() -> Result<Terminal<CrosstermBackend<Stdout>>> {
    let mut stdout = io::stdout();
    enable_raw_mode().context("failed to enable raw mode")?;
    execute!(stdout, EnterAlternateScreen).context("unable to enter alternate screen")?;
    Terminal::new(CrosstermBackend::new(stdout)).context("creating terminal failed")
}

fn restore_terminal(terminal: &mut Terminal<CrosstermBackend<Stdout>>) -> Result<()> {
    disable_raw_mode().context("failed to disable raw mode")?;
    execute!(terminal.backend_mut(), LeaveAlternateScreen)
        .context("unable to switch to main screen")?;
    terminal.show_cursor().context("unable to show cursor")
}

fn generate_output(global_state: &mut GlobalState) -> String { 
    let size = global_state.window_size;
    let mut chars = String::new();
    for i in 0..size.1 {
        for j in 0..size.0 {
            if let Some(char) = global_state.preview.get(&(j as u16, i as u16)) {
                chars.push(*char);
            } else if let Some(char) = global_state.diagram.get(&(j as u16, i as u16)) {
                chars.push(*char);
            } else {
                chars.push(' ');
            }
        }
        chars.push('\n');
    }

    return chars;
}

fn run(
    terminal: &mut Terminal<CrosstermBackend<Stdout>>,
    mut global_state: &mut GlobalState,
) -> Result<()> {
    let cursor_pos = terminal
        .get_cursor()
        .context("cannot get cursor position")?;
    let terminal_size = terminal.size().context("cannot get terminal size")?;
    loop {
        global_state.window_size = (terminal_size.width, terminal_size.height);
        // draw frame
        let completed_frame = terminal.draw(|frame| {
            let mut style = Style::default();
            if global_state.color {
                style = style.fg(Color::Yellow);
            }

            let chars = generate_output(global_state);
            let text = Paragraph::new(chars).style(style);
            frame.set_cursor(global_state.current_pos.0, global_state.current_pos.1);
            frame.render_widget(text, terminal_size);
        })?;
        // get user input
        global_state = process_input(global_state)?;
        if global_state.should_quit {
            break;
        }

        // progress global state

    }
    Ok(())
}

fn process_input(global_state: &mut GlobalState) -> Result<&mut GlobalState> {
    if event::poll(Duration::from_millis(250)).context("event poll failed")? {
        if let Event::Key(key) = event::read().context("event read failed")? {
            match key.modifiers {
                crossterm::event::KeyModifiers::CONTROL => match key.code {
                    KeyCode::Char('b') => {
                        if global_state.input_state != InputState::BOX {
                            global_state.input_state = InputState::BOX;
                            global_state.start_pos = global_state.current_pos;
                        }
                    }
                    KeyCode::Char('a') => {
                        if global_state.input_state != InputState::ARROW {
                            global_state.input_state = InputState::ARROW;
                            global_state.start_pos = global_state.current_pos;
                        }
                    }
                    KeyCode::Char('s') => {
                        let chars = generate_output(global_state);
                        let file = File::create("output.txt")?;
                        let mut buf_write = BufWriter::new(file);
                        buf_write.write(chars.as_bytes());
                        buf_write.flush().unwrap();
                    }
                    _ => {}
                },
                crossterm::event::KeyModifiers::NONE => match key.code {
                    KeyCode::Esc => global_state.should_quit = true,
                    KeyCode::Char(char) if global_state.input_state == InputState::INSERT => {
                            global_state.diagram.insert(global_state.current_pos, char);
                            if global_state.current_pos.0 < global_state.window_size.0 {
                                global_state.current_pos.0 += 1;
                            }
                    }
                    KeyCode::Backspace if global_state.input_state == InputState::INSERT => {
                        if global_state.current_pos.0 > 0 {
                            global_state.current_pos.0 -= 1;
                            global_state.diagram.remove(&global_state.current_pos);
                        }
                    }
                    KeyCode::Left => {
                        if global_state.current_pos.0 > 0 {
                            global_state.prev_pos = global_state.current_pos;
                            global_state.current_pos.0 -= 1;
                        }
                    }
                    KeyCode::Right => {
                        if global_state.current_pos.0 < global_state.window_size.0 {
                            global_state.prev_pos = global_state.current_pos;
                            global_state.current_pos.0 += 1;
                        }
                    }
                    KeyCode::Up => {
                        if global_state.current_pos.1 > 0 {
                            global_state.prev_pos = global_state.current_pos;
                            global_state.current_pos.1 -= 1;
                        }
                    }
                    KeyCode::Down => {
                        if global_state.current_pos.1 < global_state.window_size.1 {
                            global_state.prev_pos = global_state.current_pos;
                            global_state.current_pos.1 += 1;
                        }
                    }
                    KeyCode::Enter => {
                        match global_state.input_state {
                            InputState::BOX => {
                                // confirm box 
                                for pair in global_state.preview.iter() {
                                    global_state.diagram.insert(*pair.0, *pair.1);
                                }
                                global_state.preview.clear();
                                global_state.input_state = InputState::INSERT;
                            },
                            InputState::INSERT => {
                                // new line 
                                if global_state.current_pos.1 < global_state.window_size.1 {
                                    global_state.current_pos.1 += 1;
                                }
                                global_state.current_pos.0 = 0;
                            }
                            InputState::ARROW => {
                                // confirm arrow  
                                for pair in global_state.preview.iter() {
                                    global_state.diagram.insert(*pair.0, *pair.1);
                                }
                                global_state.preview.clear();
                                global_state.input_state = InputState::INSERT;
                            },
                        }
                    }
                    _ => {}
                },
                _ => {}
            }


            if global_state.input_state == InputState::BOX {
                let lefty = min(global_state.current_pos.0, global_state.start_pos.0); 
                let righty = max(global_state.current_pos.0, global_state.start_pos.0); 
                let topx = min(global_state.current_pos.1, global_state.start_pos.1); 
                let bottomx = max(global_state.current_pos.1, global_state.start_pos.1); 

                global_state.preview.clear();

                for y in lefty..righty {
                    global_state.preview.insert((y, topx), '─'); 
                    global_state.preview.insert((y, bottomx), '─'); 
                }
                for x in topx..bottomx{
                    global_state.preview.insert((lefty, x), '│'); 
                    global_state.preview.insert((righty, x), '│'); 
                }
                global_state.preview.insert((lefty, topx), '╭'); 
                global_state.preview.insert((righty, topx), '╮'); 
                global_state.preview.insert((lefty, bottomx), '╰'); 
                global_state.preview.insert((righty, bottomx), '╯'); 
            } else if global_state.input_state == InputState::ARROW {
                if global_state.prev_pos.0 + 1 == global_state.current_pos.0 { 
                    global_state.preview.insert(global_state.current_pos, '▶');
                } else if global_state.prev_pos.0 - 1 == global_state.current_pos.0 { 
                    global_state.preview.insert(global_state.current_pos, '◀');
                } else if global_state.prev_pos.1 + 1 == global_state.current_pos.1 {
                    global_state.preview.insert(global_state.current_pos, '▼');
                } else if global_state.prev_pos.1 - 1 == global_state.current_pos.1 {
                    global_state.preview.insert(global_state.current_pos, '▲');
                } else {
                    global_state.preview.insert(global_state.current_pos, '◆');
                }
                match global_state.preview.get(&global_state.prev_pos) {
                    Some('▶') => {
                        if global_state.prev_pos.0 + 1 == global_state.current_pos.0 { 
                            global_state.preview.insert(global_state.prev_pos, '─');
                        } else if global_state.prev_pos.0 - 1 == global_state.current_pos.0 { 
                            global_state.preview.insert(global_state.prev_pos, '─');
                        } else if global_state.prev_pos.1 + 1 == global_state.current_pos.1 {
                            global_state.preview.insert(global_state.prev_pos, '╮');
                        } else if global_state.prev_pos.1 - 1 == global_state.current_pos.1 {
                            global_state.preview.insert(global_state.prev_pos, '╯');
                        } else {
                            global_state.preview.insert(global_state.prev_pos, '+');
                        }
                    }
                    Some('◀') => {
                        if global_state.prev_pos.0 + 1 == global_state.current_pos.0 { 
                            global_state.preview.insert(global_state.prev_pos, '─');
                        } else if global_state.prev_pos.0 - 1 == global_state.current_pos.0 { 
                            global_state.preview.insert(global_state.prev_pos, '─');
                        } else if global_state.prev_pos.1 + 1 == global_state.current_pos.1 {
                            global_state.preview.insert(global_state.prev_pos, '╭');
                        } else if global_state.prev_pos.1 - 1 == global_state.current_pos.1 {
                            global_state.preview.insert(global_state.prev_pos, '╰');
                        } else {
                            global_state.preview.insert(global_state.prev_pos, '+');
                        }
                    }
                    Some('▼') => {
                        if global_state.prev_pos.0 + 1 == global_state.current_pos.0 { 
                            global_state.preview.insert(global_state.prev_pos,  '╰');
                        } else if global_state.prev_pos.0 - 1 == global_state.current_pos.0 { 
                            global_state.preview.insert(global_state.prev_pos,  '╯');
                        } else if global_state.prev_pos.1 + 1 == global_state.current_pos.1 {
                            global_state.preview.insert(global_state.prev_pos, '│');
                        } else if global_state.prev_pos.1 - 1 == global_state.current_pos.1 {
                            global_state.preview.insert(global_state.prev_pos, '│');
                        } else {
                            global_state.preview.insert(global_state.prev_pos, '+');
                        }
                    }
                    Some('▲') => {
                        if global_state.prev_pos.0 + 1 == global_state.current_pos.0 { 
                            global_state.preview.insert(global_state.prev_pos, '╭');
                        } else if global_state.prev_pos.0 - 1 == global_state.current_pos.0 { 
                            global_state.preview.insert(global_state.prev_pos, '╮');
                        } else if global_state.prev_pos.1 + 1 == global_state.current_pos.1 {
                            global_state.preview.insert(global_state.prev_pos, '│');
                        } else if global_state.prev_pos.1 - 1 == global_state.current_pos.1 {
                            global_state.preview.insert(global_state.prev_pos, '│');
                        } else {
                            global_state.preview.insert(global_state.prev_pos, '+');
                        }
                    }
                    Some(_) => {}
                    None => {},
                }
                // global_state.preview.insert(global_state.prev_pos, '+');
            } else {
                global_state.preview.clear();
            }
            return Ok(global_state);
        }
    }
    Ok(global_state)
}
