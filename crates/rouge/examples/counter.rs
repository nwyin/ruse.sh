use rouge::prelude::*;

struct Counter {
    count: i32,
}

impl Model for Counter {
    fn update(&mut self, msg: Msg) -> Cmd {
        if let Msg::KeyPress(key) = msg {
            match key.code {
                KeyCode::Char('q') | KeyCode::Escape => return quit(),
                KeyCode::Char('c') if key.modifiers.contains(Modifiers::CTRL) => return quit(),
                KeyCode::Up => self.count += 1,
                KeyCode::Down => self.count -= 1,
                KeyCode::Char('k') => self.count -= 1,
                KeyCode::Char('j') => self.count += 1,
                KeyCode::Char('r') => self.count = 0,
                _ => {}
            }
        }
        None
    }

    fn view(&self) -> View {
        let title_style = Style::new()
            .bold(true)
            .foreground(Color::parse("#ff6600"));

        let count_style = Style::new()
            .bold(true)
            .foreground(Color::parse("#04B575"))
            .padding(&[0, 1]);

        let help_style = Style::new().faint(true);

        let content = format!(
            "{}\n\n  {}{}\n\n{}",
            title_style.render(&["Rouge Counter"]),
            "Count:",
            count_style.render(&[&self.count.to_string()]),
            help_style.render(&["j/k or ↑/↓: change • r: reset • q: quit"]),
        );

        let box_style = Style::new()
            .border(ROUNDED_BORDER, &[true])
            .border_foreground(Color::parse("#874BFD"))
            .padding(&[1, 2]);

        View::new(box_style.render(&[&content])).with_alt_screen()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let model = Counter { count: 0 };
    let final_model = Program::new(model).with_alt_screen().run().await?;
    std::println!("Final count: {}", final_model.count);
    Ok(())
}
