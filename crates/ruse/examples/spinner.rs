use ruse::prelude::*;

struct SpinnerDemo {
    spinners: Vec<(String, Spinner)>,
}

impl Model for SpinnerDemo {
    fn init(&mut self) -> Cmd {
        // Initialize all spinners — collect their init commands
        let cmds: Vec<Cmd> = self.spinners.iter().map(|(_, s)| s.init()).collect();
        batch(cmds)
    }

    fn update(&mut self, msg: Msg) -> Cmd {
        if let Msg::KeyPress(key) = &msg {
            match key.code {
                KeyCode::Char('q') | KeyCode::Escape => return quit(),
                KeyCode::Char('c') if key.modifiers.contains(Modifiers::CTRL) => return quit(),
                _ => {}
            }
        }

        // Route to all spinners
        let cmds: Vec<Cmd> = self
            .spinners
            .iter_mut()
            .map(|(_, s)| s.update(&msg))
            .collect();
        batch(cmds)
    }

    fn view(&self) -> View {
        let title = Style::new()
            .bold(true)
            .foreground(Color::parse("#ff6600"))
            .render(&["Spinner Gallery"]);

        let mut lines = vec![title, String::new()];

        for (name, spinner) in &self.spinners {
            let label = Style::new().faint(true).render(&[name.as_str()]);
            lines.push(format!("  {} {}", spinner.view(), label));
        }

        lines.push(String::new());
        lines.push(Style::new().faint(true).render(&["Press q to quit"]));

        let box_style = Style::new()
            .border(ROUNDED_BORDER, &[true])
            .border_foreground(Color::parse("#874BFD"))
            .padding(&[1, 2]);

        View::new(box_style.render(&[&lines.join("\n")])).with_alt_screen()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let spinners = vec![
        ("Line".to_string(), Spinner::new(line_spinner())),
        ("Dot".to_string(), Spinner::new(dot_spinner())),
        ("Mini Dot".to_string(), Spinner::new(mini_dot_spinner())),
        ("Pulse".to_string(), Spinner::new(pulse_spinner())),
        ("Globe".to_string(), Spinner::new(globe_spinner())),
        ("Moon".to_string(), Spinner::new(moon_spinner())),
        ("Ellipsis".to_string(), Spinner::new(ellipsis_spinner())),
    ];

    let model = SpinnerDemo { spinners };
    Program::new(model).with_alt_screen().run().await?;
    Ok(())
}
