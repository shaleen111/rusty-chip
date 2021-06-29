mod emulator;
mod fonts;
mod machine;

use emulator::Emulator;
use machine::Chip8;

fn main()
{
    let e = &mut Emulator::new(Chip8::new(), 10.0);
    e.load("Space Invaders [David Winter].ch8");
    e.create_display();
}
