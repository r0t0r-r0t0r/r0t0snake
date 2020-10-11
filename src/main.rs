use std::path::Path;

use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::rect::Rect;
use std::time::Instant;
use std::collections::VecDeque;
use rand::Rng;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Direction {
    Up,
    Right,
    Down,
    Left,
}

impl From<u32> for Direction {
    fn from(x: u32) -> Direction {
        match x % 4 {
            0 => Direction::Up,
            1 => Direction::Right,
            2 => Direction::Down,
            3 => Direction::Left,
            _ => panic!(),
        }
    }
}

impl From<Direction> for u32 {
    fn from(x: Direction) -> u32 {
        match x {
            Direction::Up => 0,
            Direction::Right => 1,
            Direction::Down => 2,
            Direction::Left => 3,
        }
    }
}

impl Direction {
    fn cw(&self) -> Direction {
        let x: u32 = (*self).into();
        x.overflowing_add(1).0.into()
    }

    fn ccw(&self) -> Direction {
        let x: u32 = (*self).into();
        x.overflowing_sub(1).0.into()
    }

    fn is_opposite(&self, direction: Direction) -> bool {
        let d1: u32 = (*self).into();
        let d2: u32 = direction.into();

        let delta_dir: Direction = d1.overflowing_sub(d2).0.into();

        delta_dir == Direction::Down
    }
}

struct Latch {
    prev: bool,
    curr: bool,
}

impl Latch {
    fn new() -> Latch {
        Latch {
            prev: false,
            curr: false,
        }
    }

    fn set(&mut self, value: bool) {
        self.curr = value;
    }

    fn front_edge(&self) -> bool {
        self.curr && ! self.prev
    }

    fn tick(&mut self) {
        self.prev = self.curr;
    }
}

struct ScreenBuffer {
    chars: Vec<u8>,
    width: u32,
    height: u32,
}

impl ScreenBuffer {
    fn new(width: u32, height: u32) -> ScreenBuffer {
        ScreenBuffer {
            chars: vec![0; width as usize * height as usize],
            width,
            height,
        }
    }

    fn index(&self, x: u32, y: u32) -> usize {
        assert!(x < self.width);
        assert!(y < self.height);

        (y * self.width + x) as usize
    }

    fn clear(&mut self) {
        for c in self.chars.iter_mut() {
            *c = 0;
        }
    }
}

fn print(buf: &mut ScreenBuffer, x: u32, y: u32, s: &[u8]) {
    let index = buf.index(x, y);
    let len = s.len();

    buf.chars[index..(index + len)].copy_from_slice(s);
}

#[derive(Eq, PartialEq)]
enum GameState {
    Menu,
    Play,
    Pause,
    GameOver,
    Quit,
}

struct LevelBounds {
    x: u32,
    y: u32,
    width: u32,
    height: u32,
}

impl LevelBounds {
    fn new(x: u32, y: u32, width: u32, height: u32) -> LevelBounds {
        LevelBounds {
            x, y, width, height,
        }
    }

    fn draw(&self, screen_buffer: &mut ScreenBuffer) {
        let wall_chr = [0xb1u8];

        for x in self.x..(self.x + self.width) {
            print(screen_buffer, x, self.y, &wall_chr);
        }
        for y in self.y..(self.y + self.height) {
            print(screen_buffer, self.x, y, &wall_chr);
        }
        for x in self.x..(self.x + self.width) {
            print(screen_buffer, x, self.y + self.height - 1, &wall_chr);
        }
        for y in self.y..(self.y + self.height) {
            print(screen_buffer, self.x + self.width - 1, y, &wall_chr);
        }
    }

    fn is_inside(&self, x: u32, y: u32) -> bool {
        x > self.x && x < (self.x + self.width - 1) && y > self.y && y < (self.y + self.height - 1)
    }
}

struct Snake {
    body: VecDeque<(u32, u32)>,
    prev_direction: Direction,
    direction: Direction,
    period: u32,
    tick: u32,
    score: u32,
    dead: bool,
}

#[derive(Eq, PartialEq)]
enum SnakeCollision {
    Head,
    Tail,
}

impl Snake {
    fn new() -> Snake {
        let mut body: VecDeque<(u32, u32)> = VecDeque::new();
        body.push_back((10, 3));
        body.push_back((9, 3));
        body.push_back((8, 3));
        body.push_back((7, 3));
        body.push_back((6, 3));
        body.push_back((5, 3));
        body.push_back((4, 3));
        body.push_back((3, 3));

        let prev_direction = Direction::Right;
        let direction = Direction::Right;
        let period = 24;
        //let period = 48;
        //let period = 2;
        //let period = 120;
        let tick = 0;
        let score = 0;
        let dead = false;

        Snake {
            body,
            prev_direction,
            direction,
            period,
            tick,
            score,
            dead,
        }
    }

    fn is_collision(&self, x: u32, y: u32) -> Option<SnakeCollision> {
        for (i, p) in self.body.iter().copied().enumerate() {
            if i == 0 {
                if p == (x, y) {
                    return Some(SnakeCollision::Head);
                }
            }
            if p == (x, y) {
                return Some(SnakeCollision::Tail);
            }
        }

        None
    }

    fn grow(&mut self, n: u32) {
        let back = self.body.back().unwrap().clone();

        for _ in 0..n {
            self.body.push_back(back);
        }
    }

    fn move_up(&mut self) {
        if !self.prev_direction.is_opposite(Direction::Up) {
            self.direction = Direction::Up;
        }
    }

    fn move_right(&mut self) {
        if !self.prev_direction.is_opposite(Direction::Right) {
            self.direction = Direction::Right;
        }
    }

    fn move_down(&mut self) {
        if !self.prev_direction.is_opposite(Direction::Down) {
            self.direction = Direction::Down;
        }
    }

    fn move_left(&mut self) {
        if !self.prev_direction.is_opposite(Direction::Left) {
            self.direction = Direction::Left;
        }
    }

    fn update(world: &mut World) {
        if world.snake.dead {
            return;
        }

        world.snake.tick = world.snake.tick + 1;
        if world.snake.tick > world.snake.period {
            world.snake.tick = 0;

            let (x, y) = world.snake.body.front().unwrap().clone();
            
            let (new_x, new_y) = match world.snake.direction {
                Direction::Up => (x, y - 1),
                Direction::Right => (x + 1, y),
                Direction::Down => (x, y + 1),
                Direction::Left => (x - 1, y),
            };

            let object_id = world.check_collision(ObjectId::SnakeHead, new_x, new_y);

            if object_id == Some(ObjectId::LevelBound) {
                world.snake.dead = true;
            } else if object_id == Some(ObjectId::SnakeTail) {
                world.snake.dead = true;
            } else if object_id == Some(ObjectId::Apple) {
                world.snake.grow(1);
            }

            if world.snake.dead {
                return;
            }

            world.snake.body.push_front((new_x, new_y));
            world.snake.body.pop_back();

            world.snake.prev_direction = world.snake.direction;
        }
    }

    fn draw(&self, screen_buffer: &mut ScreenBuffer) {
        for (i, (x, y)) in self.body.iter().enumerate() {
            if i != 0 {
                print(screen_buffer, *x, *y, b"#");
                //print(screen_buffer, *x, *y, &[0x09u8]);

                //print(screen_buffer, *x, *y, &[0xb1u8]);
            }
        }
        let (x, y) = self.body.front().unwrap();
        print(screen_buffer, *x, *y, b"O");
        //print(screen_buffer, *x, *y, &[0x07u8]);
        //print(screen_buffer, *x, *y, b"#");
        
        //print(screen_buffer, *x, *y, &[0xdbu8]);
    }
}

struct Input {
    key_escape: Latch,
    key_enter: Latch,
    key_up: Latch,
    key_left: Latch,
    key_down: Latch,
    key_right: Latch,
}

impl Input {
    fn new() -> Input {
        Input {
            key_escape: Latch::new(),
            key_enter: Latch::new(),
            key_up: Latch::new(),
            key_left: Latch::new(),
            key_down: Latch::new(),
            key_right: Latch::new(),
        }
    }

    fn tick(&mut self) {
        self.key_escape.tick();
        self.key_enter.tick();
        self.key_up.tick();
        self.key_left.tick();
        self.key_down.tick();
        self.key_right.tick();
    }
}

struct Apple {
    pos: Option<(u32, u32)>,
}

impl Apple {
    fn new() -> Apple {
        Apple {
            pos: None,
        }
    }

    fn gen_pos(snake: &Snake, level_bounds: &LevelBounds) -> (u32, u32) {
        let mut rng = rand::thread_rng();

        loop {
            let x = rng.gen_range(level_bounds.x + 1, level_bounds.x + level_bounds.width - 1);
            let y = rng.gen_range(level_bounds.y + 1, level_bounds.y + level_bounds.height - 1);

            if snake.is_collision(x, y) == None {
                return (x, y);
            }
        }
    }

    fn update(world: &mut World) {
        if world.apple.pos == None {
            world.apple.pos = Some(Self::gen_pos(&world.snake, &world.level_bounds));
        }

        if let Some((x, y)) = world.apple.pos {
            let object_id = world.check_collision(ObjectId::Apple, x, y);

            if object_id == Some(ObjectId::SnakeHead) {
                world.apple.pos = Some(Self::gen_pos(&world.snake, &world.level_bounds));
            }
        }
    }

    fn draw(&self, screen_buffer: &mut ScreenBuffer) {
        if let Some((x, y)) = self.pos {
            print(screen_buffer, x, y, b"$");
        }
    }
}

#[derive(Eq, PartialEq)]
enum ObjectId {
    SnakeHead,
    SnakeTail,
    LevelBound,
    Apple,
}

struct World {
    snake: Snake,
    level_bounds: LevelBounds,
    apple: Apple,
}

impl World {
    fn new(snake: Snake, level_bounds: LevelBounds) -> World {
        let apple = Apple::new();

        World {
            snake,
            level_bounds,
            apple,
        }
    }

    fn check_collision(&self, id: ObjectId, x: u32, y: u32) -> Option<ObjectId> {
        // We assume that object does not collide with itself.
        
        if id != ObjectId::LevelBound && !self.level_bounds.is_inside(x, y) {
            //self.snake.kill();
            return Some(ObjectId::LevelBound);
        }

        if let Some(snake_collision) = self.snake.is_collision(x, y) {
            if id != ObjectId::SnakeHead && snake_collision == SnakeCollision::Head {
                return Some(ObjectId::SnakeHead);
            } else if id != ObjectId::SnakeTail && snake_collision == SnakeCollision::Tail {
                return Some(ObjectId::SnakeTail);
            }
            //self.snake.kill();
        }

        if id != ObjectId::Apple && self.apple.pos == Some((x, y)) {
            //self.snake.grow(1);
            //self.apple.change_pos();
            return Some(ObjectId::Apple);
        }

        None
    }
}

fn main() -> Result<(), String> {
    let scale = 2;
    let tile_count = (30, 20);
    let tile_size = (24, 24);

    sdl2::hint::set("SDL_VIDEO_X11_NET_WM_BYPASS_COMPOSITOR", "0");

    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;

    let window = video_subsystem.window("SDL2",
                    scale * tile_count.0 * tile_size.0,
                    scale * tile_count.1 * tile_size.1)
        .position_centered().build().map_err(|e| e.to_string())?;

    let mut canvas = window.into_canvas()
        .accelerated().build().map_err(|e| e.to_string())?;
    let texture_creator = canvas.texture_creator();

    canvas.set_draw_color(sdl2::pixels::Color::RGBA(0,0,0,255));

    let mut event_pump = sdl_context.event_pump()?;

    let tileset_surface = sdl2::surface::Surface::load_bmp(Path::new("assets/tileset_24_24.bmp"))?;
    let tileset_texture = texture_creator.create_texture_from_surface(&tileset_surface)
        .map_err(|e| e.to_string())?;

    let mut tileset_src_rect = Rect::new(16, 0, tile_size.0, tile_size.1);
    let mut tileset_dst_rect = Rect::new(0, 0, tile_size.0 * scale, tile_size.1 * scale);

    let mut screen_buffer: ScreenBuffer = ScreenBuffer::new(tile_count.0, tile_count.1);

    let mut input = Input::new();

    let update_period = 1.0 / 120.0;
    let mut update_now = Instant::now();

    let draw_period = 1.0 / 60.0;
    let mut draw_now = Instant::now();

    let snake: Snake = Snake::new();

    let level_bounds = LevelBounds::new(0, 0, 30, 20);

    //let mut apple = Apple::new(&snake, &level_bounds);

    let mut world = World::new(snake, level_bounds);

    let mut state = GameState::Play;
    //let mut state = GameState::GameOver;
    while state != GameState::Quit {
        let new_update_now = Instant::now();
        if (new_update_now - update_now).as_secs_f64() >= update_period {
            update_now = new_update_now;

            for event in event_pump.poll_iter() {
                match event {
                    Event::Quit {..}  => {
                        state = GameState::Quit;
                    },
                    Event::KeyDown {scancode: Some(Scancode::Escape), ..} => {
                        input.key_escape.set(true);
                    },
                    Event::KeyUp {scancode: Some(Scancode::Escape), ..} => {
                        input.key_escape.set(false);
                    },
                    Event::KeyDown {scancode: Some(Scancode::Return), ..} => {
                        input.key_enter.set(true);
                    },
                    Event::KeyUp {scancode: Some(Scancode::Return), ..} => {
                        input.key_enter.set(false);
                    },
                    Event::KeyDown {scancode: Some(Scancode::Up), ..} => {
                        input.key_up.set(true);
                    },
                    Event::KeyUp {scancode: Some(Scancode::Up), ..} => {
                        input.key_up.set(false);
                    },
                    Event::KeyDown {scancode: Some(Scancode::Left), ..} => {
                        input.key_left.set(true);
                    },
                    Event::KeyUp {scancode: Some(Scancode::Left), ..} => {
                        input.key_left.set(false);
                    },
                    Event::KeyDown {scancode: Some(Scancode::Down), ..} => {
                        input.key_down.set(true);
                    },
                    Event::KeyUp {scancode: Some(Scancode::Down), ..} => {
                        input.key_down.set(false);
                    },
                    Event::KeyDown {scancode: Some(Scancode::Right), ..} => {
                        input.key_right.set(true);
                    },
                    Event::KeyUp {scancode: Some(Scancode::Right), ..} => {
                        input.key_right.set(false);
                    },
                    _ => {},
                }
            }

            if state == GameState::Play {
                if input.key_up.front_edge() {
                    world.snake.move_up();
                }
                if input.key_right.front_edge() {
                    world.snake.move_right();
                }
                if input.key_down.front_edge() {
                    world.snake.move_down();
                }
                if input.key_left.front_edge() {
                    world.snake.move_left()
                }

                Snake::update(&mut world);
                if world.snake.dead {
                    state = GameState::GameOver;
                } else {
                    Apple::update(&mut world);
                }
            } else if state == GameState::GameOver {
                if input.key_escape.front_edge() {
                    state = GameState::Quit;
                }
            }

            input.tick();
        }

        let new_draw_now = Instant::now();
        if (new_draw_now - draw_now).as_secs_f64() >= draw_period {
            draw_now = new_draw_now;

            // render chars
            screen_buffer.clear();

            if state == GameState::Play {
                world.level_bounds.draw(&mut screen_buffer);
                world.apple.draw(&mut screen_buffer);
                world.snake.draw(&mut screen_buffer);
            }
            if state == GameState::GameOver {
                print(&mut screen_buffer, 0, 0, b"Game over!");
            }
            

            canvas.clear();
            for y in 0..tile_count.1 {
                for x in 0..tile_count.0 {
                    let chr = screen_buffer.chars[(y * tile_count.0 + x) as usize];

                    tileset_src_rect.set_x(((chr as usize % 16) * tile_size.0 as usize) as i32);
                    tileset_src_rect.set_y(((chr as usize / 16) * tile_size.1 as usize) as i32);

                    tileset_dst_rect.set_x((x * tile_size.0 * scale) as i32);
                    tileset_dst_rect.set_y((y * tile_size.1 * scale) as i32);

                    canvas.copy_ex(&tileset_texture, Some(tileset_src_rect), Some(tileset_dst_rect), 0.0, None, false, false)?;
                }
            }
            canvas.present();
        }
    }

    Ok(())
}
