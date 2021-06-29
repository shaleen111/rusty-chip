use ggez::{conf,
           Context, ContextBuilder,
           event,
           graphics,
           timer};

use crate::machine::{self, Chip8};

pub struct Emulator
{
    machine: Chip8,

    scale: f32,
    width: f32,
    height: f32,

    cycles_per_sec: u32,

    window_title: String,
}

impl Emulator
{
    pub fn new(machine: Chip8, scale: f32) -> Emulator
    {
        Emulator
        {
            machine,

            scale,
            width: scale * machine::VIDEO_WIDTH as f32,
            height: scale * machine::VIDEO_HEIGHT as f32,

            cycles_per_sec: 60,
            window_title: String::from("Chip-8 Emulator"),
        }
    }

    pub fn load(&mut self, path: &str)
    {
        self.machine.load(path);
    }

    pub fn create_display(&mut self)
    {
        let (ctx, event_loop) = &mut ContextBuilder::new("Chip-8 Emulator", "Shaleen Baral")
                                        .window_setup(conf::WindowSetup::default().title(&self.window_title))
                                        .window_mode(conf::WindowMode::default().dimensions(self.width, self.height))
                                        .build().expect("Error Creating Context!");

        event::run(ctx, event_loop, self).expect("Error Running Emulator");
    }

    fn draw_pixel(&self, buffer_index: usize, ctx: &mut Context)
    {
        let y = (buffer_index / machine::VIDEO_WIDTH) as f32 * self.scale;
        let x = (buffer_index % machine::VIDEO_WIDTH) as f32 * self.scale;

        let r = graphics::Rect::new(x, y, self.scale, self.scale);
        let mesh_r = graphics::Mesh::new_rectangle(ctx, graphics::DrawMode::fill(),
                                                   r, graphics::Color::new(0.0, 1.0, 0.0, 1.0))
                                     .expect("Error Creating Mesh");

        graphics::draw(ctx, &mesh_r, graphics::DrawParam::default()).expect("Error Drawing From Video Buffer");
    }
}

impl event::EventHandler for Emulator
{
    fn update(&mut self, ctx: &mut Context) -> ggez::GameResult
    {
        while timer::check_update_time(ctx, self.cycles_per_sec)
        {
            self.machine.cycle();
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> ggez::GameResult
    {
        if self.machine.redraw
        {
            graphics::clear(ctx, graphics::Color::new(0.0, 0.0, 0.0, 1.0));

            for i in 0..self.machine.video.len()
            {
                if self.machine.video[i]
                {
                    self.draw_pixel(i, ctx);
                }
            }
        }
        graphics::present(ctx).expect("Error Presenting");

        Ok(())
    }
}
