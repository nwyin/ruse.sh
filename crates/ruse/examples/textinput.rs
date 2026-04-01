use ruse::prelude::*;

struct Form {
    inputs: Vec<TextInput>,
    focus_index: usize,
    submitted: bool,
}

impl Model for Form {
    fn init(&mut self) -> Cmd {
        self.inputs[0].focus()
    }

    fn update(&mut self, msg: Msg) -> Cmd {
        if let Msg::KeyPress(key) = &msg {
            match key.code {
                KeyCode::Char('c') if key.modifiers.contains(Modifiers::CTRL) => return quit(),
                KeyCode::Escape => return quit(),
                KeyCode::Tab | KeyCode::Enter => {
                    if self.submitted {
                        return quit();
                    }

                    // Check if all fields are filled on Enter
                    if key.code == KeyCode::Enter {
                        let all_filled = self.inputs.iter().all(|i| !i.value().is_empty());
                        if all_filled {
                            self.submitted = true;
                            return None;
                        }
                    }

                    // Move focus to next input
                    self.inputs[self.focus_index].blur();
                    self.focus_index = (self.focus_index + 1) % self.inputs.len();
                    return self.inputs[self.focus_index].focus();
                }
                KeyCode::BackTab => {
                    // Move focus to previous input
                    self.inputs[self.focus_index].blur();
                    if self.focus_index == 0 {
                        self.focus_index = self.inputs.len() - 1;
                    } else {
                        self.focus_index -= 1;
                    }
                    return self.inputs[self.focus_index].focus();
                }
                _ => {}
            }
        }

        // Route to focused input
        if !self.submitted {
            self.inputs[self.focus_index].update(&msg)
        } else {
            None
        }
    }

    fn view(&self) -> View {
        let title = Style::new()
            .bold(true)
            .foreground(Color::parse("#ff6600"))
            .render(&["User Registration"]);

        let mut content = vec![title, String::new()];

        if self.submitted {
            let success = Style::new()
                .bold(true)
                .foreground(Color::parse("#04B575"))
                .render(&["Submitted!"]);
            content.push(success);
            content.push(String::new());

            for input in &self.inputs {
                content.push(format!("  {}", input.value()));
            }

            content.push(String::new());
            content.push(Style::new().faint(true).render(&["Press any key to quit"]));
        } else {
            for (i, input) in self.inputs.iter().enumerate() {
                let indicator = if i == self.focus_index {
                    Style::new()
                        .foreground(Color::parse("#874BFD"))
                        .render(&[">"])
                } else {
                    " ".to_string()
                };
                content.push(format!("{} {}", indicator, input.view()));
            }

            content.push(String::new());
            content.push(
                Style::new()
                    .faint(true)
                    .render(&["tab: next field • enter: submit • esc: quit"]),
            );
        }

        let box_style = Style::new()
            .border(ROUNDED_BORDER, &[true])
            .border_foreground(Color::parse("#874BFD"))
            .padding(&[1, 2]);

        View::new(box_style.render(&[&content.join("\n")])).with_alt_screen()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Form {
        inputs: vec![
            TextInput::new()
                .with_prompt("Name: ")
                .with_placeholder("John Doe")
                .with_width(40),
            TextInput::new()
                .with_prompt("Email: ")
                .with_placeholder("john@example.com")
                .with_width(40),
            TextInput::new()
                .with_prompt("Password: ")
                .with_placeholder("********")
                .with_echo_mode(ruse::components::textinput::EchoMode::Password)
                .with_width(40),
        ],
        focus_index: 0,
        submitted: false,
    };

    Program::new(model).with_alt_screen().run().await?;
    Ok(())
}
