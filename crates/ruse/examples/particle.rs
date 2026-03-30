// Particle animation demo — a projectile launched with horizontal velocity
// falls under terminal gravity.  Port of charm.sh/harmonica's particle example.

use std::time::Duration;

use ruse::prelude::*;

const FPS: u32 = 60;

struct Frame;

struct ParticleDemo {
    projectile: ruse::harmonica::Projectile,
    pos: ruse::harmonica::Point,
    width: usize,
    height: usize,
    trail: Vec<(usize, usize)>,
}

impl ParticleDemo {
    fn new() -> Self {
        let init_pos = ruse::harmonica::Point::new(2.0, 3.0, 0.0);
        let init_vel = ruse::harmonica::Point::new(30.0, -8.0, 0.0);
        let projectile = ruse::harmonica::Projectile::new(
            ruse::harmonica::fps(FPS),
            init_pos,
            init_vel,
            ruse::harmonica::TERMINAL_GRAVITY,
        );
        Self {
            pos: init_pos,
            projectile,
            width: 80,
            height: 24,
            trail: Vec::new(),
        }
    }

    fn schedule_frame() -> Cmd {
        tick(Duration::from_secs(1) / FPS, |_| Msg::custom(Frame))
    }
}

impl Model for ParticleDemo {
    fn init(&mut self) -> Cmd {
        Self::schedule_frame()
    }

    fn update(&mut self, msg: Msg) -> Cmd {
        match &msg {
            Msg::KeyPress(_) => return quit(),
            Msg::WindowSize { width, height } => {
                self.width = *width as usize;
                self.height = *height as usize;
                return None;
            }
            _ => {}
        }

        if msg.downcast_ref::<Frame>().is_some() {
            // Save trail point
            let tx = self.pos.x.round().max(0.0) as usize;
            let ty = self.pos.y.round().max(0.0) as usize;
            self.trail.push((tx, ty));

            self.pos = self.projectile.update();

            if self.pos.y > self.height as f64 || self.pos.x > self.width as f64 {
                return tick(Duration::from_millis(500), |_| Msg::Quit);
            }

            return Self::schedule_frame();
        }

        None
    }

    fn view(&self) -> View {
        let y = self.pos.y.round().max(0.0) as usize;
        let x = self.pos.x.round().max(0.0) as usize;

        let title = Style::new()
            .bold(true)
            .foreground(Color::parse("#FF6600"))
            .render(&["Particle Demo"]);

        let info = Style::new()
            .faint(true)
            .render(&[&format!("  pos=({:.1}, {:.1})  vel=({:.1}, {:.1})",
                self.pos.x, self.pos.y,
                self.projectile.velocity().x, self.projectile.velocity().y,
            )]);

        // Build a grid
        let max_y = self.height.saturating_sub(4);
        let max_x = self.width.saturating_sub(1);
        let mut grid: Vec<Vec<char>> = (0..max_y).map(|_| vec![' '; max_x]).collect();

        // Draw trail
        for &(tx, ty) in &self.trail {
            if ty < max_y && tx < max_x {
                grid[ty][tx] = '.';
            }
        }

        // Draw ball
        if y < max_y && x < max_x {
            grid[y][x] = '@';
        }

        // Render grid with styling
        let trail_style = Style::new().foreground(Color::parse("#555555"));
        let ball_style = Style::new()
            .bold(true)
            .foreground(Color::parse("#FF6600"));

        let mut lines = Vec::new();
        for row in &grid {
            let mut line = String::new();
            for &ch in row {
                match ch {
                    '@' => line.push_str(&ball_style.render(&["@"])),
                    '.' => line.push_str(&trail_style.render(&["."])),
                    _ => line.push(' '),
                }
            }
            lines.push(line);
        }

        let help = Style::new()
            .faint(true)
            .render(&["  Press any key to quit"]);

        let content = format!(
            "  {title}  {info}\n{}\n{help}",
            lines.join("\n")
        );

        View::new(content).with_alt_screen()
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    Program::new(ParticleDemo::new())
        .with_alt_screen()
        .run()
        .await?;
    Ok(())
}
