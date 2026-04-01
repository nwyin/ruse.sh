// Spring animation demo — a sprite bounces toward a target using a
// damped harmonic oscillator.  Port of charm.sh/harmonica's TUI example.

use std::time::Duration;

use ruse::prelude::*;

const FPS: u32 = 60;
const SPRITE_WIDTH: usize = 12;
const SPRITE_HEIGHT: usize = 5;
const FREQUENCY: f64 = 7.0;
const DAMPING: f64 = 0.15; // under-damped — will overshoot and bounce

struct Frame;

struct SpringDemo {
    x: f64,
    x_vel: f64,
    target_x: f64,
    spring: ruse::harmonica::Spring,
    settled: bool,
}

impl SpringDemo {
    fn new() -> Self {
        Self {
            x: 0.0,
            x_vel: 0.0,
            target_x: 50.0,
            spring: ruse::harmonica::Spring::new(ruse::harmonica::fps(FPS), FREQUENCY, DAMPING),
            settled: false,
        }
    }

    fn schedule_frame() -> Cmd {
        tick(Duration::from_secs(1) / FPS, |_| Msg::custom(Frame))
    }
}

impl Model for SpringDemo {
    fn init(&mut self) -> Cmd {
        Self::schedule_frame()
    }

    fn update(&mut self, msg: Msg) -> Cmd {
        match &msg {
            Msg::KeyPress(_) => return quit(),
            _ => {}
        }

        if msg.downcast_ref::<Frame>().is_some() {
            if self.settled {
                return quit();
            }

            (self.x, self.x_vel) = self.spring.update(self.x, self.x_vel, self.target_x);

            if (self.x - self.target_x).abs() < 0.01 && self.x_vel.abs() < 0.01 {
                self.settled = true;
                return tick(Duration::from_millis(800), |_| Msg::custom(Frame));
            }

            return Self::schedule_frame();
        }

        None
    }

    fn view(&self) -> View {
        let x = self.x.round().max(0.0) as usize;

        let title = Style::new()
            .bold(true)
            .foreground(Color::parse("#FF6600"))
            .render(&["Spring Demo"]);

        let info = Style::new().faint(true).render(&[&format!(
            "  x={:.1}  vel={:.1}  target={}  freq={}  damping={}",
            self.x, self.x_vel, self.target_x, FREQUENCY, DAMPING
        )]);

        let sprite_inner = "/".repeat(SPRITE_WIDTH);
        let sprite_style = Style::new()
            .foreground(Color::parse("#FFFDF5"))
            .background(Color::parse("#575BD8"));
        let sprite_row = format!("{}{}", " ".repeat(x), sprite_style.render(&[&sprite_inner]));

        let sprite_block = (0..SPRITE_HEIGHT)
            .map(|_| sprite_row.as_str())
            .collect::<Vec<_>>()
            .join("\n");

        let help = Style::new()
            .faint(true)
            .render(&["  Press any key to quit"]);

        let content = format!("\n  {title}\n  {info}\n\n{sprite_block}\n\n{help}\n");

        View::new(content).with_alt_screen()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Program::new(SpringDemo::new())
        .with_alt_screen()
        .run()
        .await?;
    Ok(())
}
