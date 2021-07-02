use ggez::{conf,
    Context, ContextBuilder,
    event,
    graphics,
    input::{self, keyboard::KeyCode},
    timer};

use crate::machine::{self, Chip8};

const BG_COLOR: graphics::Color = graphics::Color::new(0.0, 0.0, 0.0, 1.0);

pub struct Emulator
{
    machine: Chip8,

    scale: f32,
    width: f32,
    height: f32,

    frame: [u8; 4 * machine::VIDEO_HEIGHT * machine::VIDEO_WIDTH],

    controls: [input::keyboard::KeyCode; machine::NUM_KEYS],

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

            frame: [255; 4 * machine::VIDEO_WIDTH * machine::VIDEO_HEIGHT],

            controls: [KeyCode::Key1, KeyCode::Key2, KeyCode::Key3, KeyCode::Key4,
                       KeyCode::Q,    KeyCode::W,    KeyCode::E,    KeyCode::R,
                       KeyCode::A,    KeyCode::S,    KeyCode::D,    KeyCode::F,
                       KeyCode::Z,    KeyCode::X,    KeyCode::C,    KeyCode::V],

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
                                        .window_setup(conf::WindowSetup::default().title(&self.window_title).vsync(true))
                                        .window_mode(conf::WindowMode::default().dimensions(self.width, self.height))
                                        .build().expect("Error Creating Context!");

        event::run(ctx, event_loop, self).expect("Error Running Emulator");
    }

    fn update_buffer(&mut self)
    {
        for y in 0..machine::VIDEO_HEIGHT
        {
            for x in 0..machine::VIDEO_WIDTH
            {
                let index = y * machine::VIDEO_WIDTH + x;
                let start = 4 * index;

                if self.machine.video[index]
                {
                    self.frame[start] = 255;
                    self.frame[start + 1] = 255;
                    self.frame[start + 2] = 255;
                }
                else
                {
                    self.frame[start] = 0;
                    self.frame[start + 1] = 0;
                    self.frame[start + 2] = 0;
                }
            }
        }
    }

    fn display_buffer(&self, ctx: &mut Context)
    {
        // Perhaps you could store frame_image and update it only when
        // the buffer updates but the performance is already so good that
        // the added memory overhead may not be worth it
        // and be also be slightly annoying to implement since we don't have
        // a ggez Context when the struct is initialized from Emulator::new()
        let mut frame_image = graphics::Image::from_rgba8(ctx,
                                machine::VIDEO_WIDTH as u16,
                                machine::VIDEO_HEIGHT as u16,
                                &self.frame)
                                .expect("Error Creating Frame");

        frame_image.set_filter(graphics::FilterMode::Nearest);

        graphics::draw(ctx,
                       &frame_image,
                       graphics::DrawParam::default().scale([self.scale, self.scale]))
                       .expect("Error Drawing Frame");
    }
}

impl event::EventHandler for Emulator
{
    fn update(&mut self, ctx: &mut Context) -> ggez::GameResult
    {

        while timer::check_update_time(ctx, 60)
        {
            for _i in 0..8
            {
                self.machine.fetch_and_execute();
            }
            self.machine.decrement_timers();
        }

        Ok(())
    }

    fn draw(&mut self, ctx: &mut Context) -> ggez::GameResult
    {
        graphics::clear(ctx, BG_COLOR);

        if self.machine.redraw
        {
            self.update_buffer();
        }

        self.display_buffer(ctx);

        graphics::present(ctx).expect("Error Presenting");

        Ok(())
    }

    fn key_down_event(&mut self, _ctx: &mut Context, keycode: input::keyboard::KeyCode, _keymods: input::keyboard::KeyMods, repeat: bool)
    {
        if repeat
        {
            return;
        }

        for i in 0..machine::NUM_KEYS
        {
            if self.controls[i] == keycode
            {
                self.machine.keypad[i] = true;
                return;
            }
        }
    }

    fn key_up_event(&mut self,_ctx: &mut Context, keycode: input::keyboard::KeyCode, _keymods: input::keyboard::KeyMods)
    {

        for i in 0..machine::NUM_KEYS
        {
            if self.controls[i] == keycode
            {
                self.machine.keypad[i] = false;
                return;
            }

        }
    }
}
