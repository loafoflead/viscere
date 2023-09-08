use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::keyboard::Keycode;
use sdl2::render::TextureQuery;
use sdl2::rect::Rect;

use std::time::Duration;
use std::cmp::{max, min};

mod grid;
use grid::*;
mod vec2;
use vec2::*;

const WINDOW_WIDTH      : usize = 800;
const WINDOW_HEIGHT     : usize = 600;

const CURSOR_COLOUR     : Color = Color::RGB(200, 200, 200);
const MAX_CURSOR_SIZE   : usize = 10;

const TEXT_COLOUR       : Color = Color::RGBA(255, 255, 255, 255);
const DEBUG_DRAW_COLOUR : Color = Color::RGBA(255, 0, 0, 255);
const DEFAULT_FONT      : &str  = "/usr/share/fonts/truetype/lato/Lato-Medium.ttf";

const PLAYER_WIDTH      : u32 = TILE_WIDTH as u32;
const PLAYER_HEIGHT     : u32 = TILE_WIDTH as u32 * 2u32;

const GROUNDED_COLLIDER_WIDTH: u32 = PLAYER_WIDTH;
const GROUNDED_COLLIDER_HEIGHT: u32 = PLAYER_HEIGHT / 3;

const PLAYER_DECELERATION: f32 = 0.85;

const GROUND_FRICTION: f32 = 0.65;
const HORIZ_AIR_DECELERATION: f32 = 0.4;
const VERT_AIR_DECELERATION: f32 = 0.85;

const GRAVITY: f32 = 3.0;
const PLAYER_HORIZONTAL_MOVEMENT_SPEED: f32 = 5.0;
const COLLISION_SUBSTEPS: usize = 5;

const MAXJUMP: f32 = 6.0;
const JUMP_DECR: f32 = 0.5;

const PLAYER_COLOUR     : Color = Color::RGB(10, 50, 200);

/// Stupid but this is how many frames must pass for the simulation to tick. So 2 means after 2
/// frames the simulation updates.
const SIMULATION_FRAME_DELAY    : usize  = 2;
const FPS                       : u32    = 60;

#[derive(Debug)]
struct TileId {
    name: &'static str,
    // u32 > 0xRR_GG_BB_AA
    colour      : (u8, u8, u8),
    gravity     : bool,
    flammable   : bool,
    solid       : bool,
// FIXME: make weight a substitute for solidity
    weight      : f32,
    friction    : f32,
    sort        : TileIdType,
    neighbours  : &'static [Neighbour],
}

impl TileId {
    const fn default() -> Self {
        Self {
            name: "!ERROR!", 
            colour: (255, 0, 0),
            gravity: false,
            flammable: false,
            solid: true,
            weight: 1.0,
            friction: 1.0,
            sort: TileIdType::Static,
            neighbours: &[],
        }
    }
}



const AIR_TILE: TileId = TileId {
    name        : "Air",
    colour      : (24, 24, 24),
    gravity     : false,
    flammable   : false,
    solid       : false,
    weight      : 1.0, 
    friction    : 0.0,
    sort        : TileIdType::Static,
    neighbours  : &[],
};

const AIR           : usize = 0;
const WOOD          : usize = 1;
const STONE         : usize = 2;
const SAND          : usize = 3;
const GRAVEL        : usize = 4;

const TILES: &[TileId] = { 
    use Neighbour::*;
    &[
        AIR_TILE,
        TileId { 
            name: "Wood",
            colour: (164, 42, 42),
            flammable: true,
            ..TileId::default()
        },
        TileId {
            name: "Stone", 
            colour: (180, 170, 180),
            ..TileId::default()
        },
        TileId {
            name: "Sand", 
            colour: (255, 255, 0),
            gravity: true,
            sort: TileIdType::Dynamic,
            neighbours: &[Down, DownLeft, DownRight],
            ..TileId::default()
        },
        TileId {
            name: "Gravel", 
            colour: (90, 89, 88),
            gravity: true,
            sort: TileIdType::Static,
            neighbours: &[Down, DownLeft, DownRight],
            ..TileId::default()
        },
        TileId {
            name: "Smoke", 
            colour: (244, 234, 250),
            gravity: true,
            weight: -0.5,
            friction: 0.0,
            solid: false,
            sort: TileIdType::Dynamic,
            neighbours: &[Up, UpLeft, UpRight, Left, Right],
            ..TileId::default()
        },
        TileId {
            name: "Water",
            colour: (0, 0, 255),
            gravity: true,
            weight: 0.8,
            solid: false,
            neighbours: &[Down, DownLeft, DownRight, Left, Right],
            ..TileId::default()
        }
    ]
};

type Canvas2 = sdl2::render::Canvas<sdl2::video::Window>;

fn draw_cursor(mut x: usize, mut y: usize, canvas: &mut Canvas2, size: usize) {
    let rect = if size == 1 {
        Rect::new(x as i32 * TILE_WIDTH as i32, y as i32 * TILE_HEIGHT as i32, TILE_WIDTH as u32, TILE_HEIGHT as u32)
    } else {
        if x.checked_sub(size/2).is_none() { x = size/2; }
        if y.checked_sub(size/2).is_none() { y = size/2; }
        Rect::new((x - size/2) as i32 * TILE_WIDTH as i32, (y - size/2) as i32 * TILE_HEIGHT as i32, (TILE_WIDTH * size) as u32, (TILE_HEIGHT*size) as u32)
    };
    canvas.set_draw_color(CURSOR_COLOUR);
    let _ = canvas.fill_rect(rect);
}

fn texture_and_rect_from_str<'a>(ttf_ctx: &'a sdl2::ttf::Sdl2TtfContext, texture_creator: &'a sdl2::render::TextureCreator<sdl2::video::WindowContext>, text: &str, font: &str, font_size: u16, colour: Color) -> (sdl2::render::Texture<'a>, Rect) {
    let mut font = ttf_ctx.load_font(font, font_size).unwrap();
    font.set_style(sdl2::ttf::FontStyle::BOLD);

    let surface = font
        .render(text)
        .blended(colour)
        .map_err(|e| e.to_string()).unwrap();
    let texture = texture_creator.create_texture_from_surface(&surface).unwrap();

    let TextureQuery { width, height, .. } = texture.query();
    let target = Rect::new(0, 0, width as u32, height as u32);

    (texture, target)
}

#[derive(Default, Debug)]
struct Player {
    pos: Vec2,
    vel: Vec2,
    acc: Vec2
}

impl Player {
    fn draw(&self, canvas: &mut Canvas2) {
        let rect = Rect::new(self.pos.0 as i32, self.pos.1 as i32, PLAYER_WIDTH, PLAYER_HEIGHT);
        canvas.set_draw_color(PLAYER_COLOUR);
        canvas.fill_rect(rect);    
    }

    fn rect(&self) -> Rect {
        Rect::new(self.pos.0.ceil() as i32, self.pos.1.ceil() as i32, PLAYER_WIDTH, PLAYER_HEIGHT)
    }

    fn is_grounded(&self, grid: &Grid) -> bool {
        let col_rect = Rect::new(self.pos.0 as i32 + PLAYER_WIDTH as i32/8, self.pos.1 as i32 + PLAYER_HEIGHT as i32, GROUNDED_COLLIDER_WIDTH - PLAYER_WIDTH/8, GROUNDED_COLLIDER_HEIGHT);
        if let Some(cols) = grid.get_cols_in_rect(col_rect) {
            if cols.len() > 0 {
                true
            }
            else { false }
        }
        else {
            false
        }
    }

    fn update(&mut self, grid: &Grid) {
        let prev = self.pos;
        self.acc.1 = GRAVITY; 
        self.vel = self.vel + self.acc;
        self.vel.1 = self.vel.1 * VERT_AIR_DECELERATION;
        if self.is_grounded(grid) {
            self.vel.0 *= GROUND_FRICTION;
        }
        else {
            self.vel.0 *= HORIZ_AIR_DECELERATION;
        }
        self.acc.0 *= PLAYER_DECELERATION;

        self.pos = self.pos + self.vel;
        'substep: for (i, pt) in Vec2::linef32(prev, self.pos).into_iter().enumerate() {
            self.pos = pt;
            if let Some(cols) = grid.get_cols_in_rect(self.rect()) {
                let len = cols.len();
                'subcollisions: for col in cols {
                    let player_rect = self.rect();
                    let col_obj_rect = col;

                    let Some(intersection) = player_rect.intersection(col_obj_rect) else { continue; };

                    let pcentre: Vec2 = player_rect.center().into();
                    let ccentre = col_obj_rect.center().into();

                    let p_to_obj = pcentre - ccentre;
                    // eprintln!("{:?}", (ccentre - pcentre));
                    
                    let (x_part, y_part) = if intersection.w == intersection.h {
                        let p_to_obj = prev - self.pos;

                        (p_to_obj.0.signum() * intersection.w as f32, p_to_obj.1.signum() * intersection.h as f32)
                    } else {
                        let x_part = if intersection.w < intersection.h { p_to_obj.0.signum() * intersection.w as f32 } else { 0.0 };
                        let y_part = if intersection.h < intersection.w { p_to_obj.1.signum() * intersection.h as f32 } else { 0.0 };
                        (x_part, y_part)
                    };

                    self.pos = self.pos + Vec2(x_part, y_part);

                    if intersection.w < intersection.h { self.vel.0 = 0.0; }
                    if intersection.h < intersection.w { self.vel.1 = 0.0; }

                    // self.pos = self.pos - (pcentre - ccentre).signum() * Vec2(intersection.w as f32, intersection.h as f32);
                    //self.vel = Vec2::ZERO;
                    //self.pos = self.pos - Vec2(player_rect.w as f32 - intersection.w as f32, intersection.h as f32);
                    //self.vel = self.vel - Vec2(player_rect.w as f32 - intersection.w as f32, intersection.h as f32);
                    // self.vel = Vec2(player_rect.x as f32 - intersection.x as f32, player_rect.y as f32 - intersection.y as f32);
                }
                if len > 0 && i > 1 {
                    break 'substep;
                }
            }
        }
        /*if grid.get_solidity(self.pos.0 as usize / TILE_WIDTH, self.pos.1 as usize / TILE_HEIGHT) {
            let col_obj_rect = Rect::new(self.pos.0 as i32, self.pos.1 as i32, TILE_WIDTH as u32, TILE_HEIGHT as u32);
            let player_rect = Rect::new(self.pos.0 as i32, self.pos.1 as i32, PLAYER_WIDTH, PLAYER_HEIGHT);

            let nxl = max(player_rect.x, col_obj_rect.x);
            let nyl = max(player_rect.y + player_rect.h, col_obj_rect.y + col_obj_rect.h);
            let nxr = min(player_rect.x + player_rect.w, col_obj_rect.x + col_obj_rect.w);
            let nyr = min(player_rect.y, col_obj_rect.y);

            let intersect_pt_tl = Vec2(nxl as f32, nyr as f32);
            let intersect_pt_br = Vec2(nxr as f32, nyl as f32);

            self.vel = Vec2::ZERO;
            self.pos = intersect_pt_tl - Vec2(PLAYER_WIDTH as f32, PLAYER_HEIGHT as f32);
        }*/
    }

    fn move_x(&mut self, acc: f32) {
        self.vel.0 += acc;
    }
}

pub fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let ttf_ctx = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let window = video_subsystem.window(":(", WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32)
        .opengl()
        .position_centered()
        .build()?;

    let mut canvas = window.into_canvas().build()?;

    let texture_creator = canvas.texture_creator();

    // let (texture, target) = texture_and_rect_from_str(&ttf_ctx, &texture_creator, "hello world", DEFAULT_FONT, 24, TEXT_COLOUR);
        
    let mut grid = Grid::new(WINDOW_WIDTH / TILE_WIDTH, WINDOW_HEIGHT / TILE_HEIGHT)?;
    let mut timer = 0usize;

    // let mut grid: [[TileIndex; WINDOW_WIDTH / TILE_WIDTH]; WINDOW_HEIGHT / TILE_HEIGHT] = ;
    let mut cur_x = 0;
    let mut cur_y = 0;
    let mut cur_tile = 1;
    let mut cur_size = 2;

    let mut player = Player {
        pos: Vec2(WINDOW_WIDTH as f32 / 2.0 + 5.0, WINDOW_HEIGHT as f32 / 2.0),
        ..Default::default()
    };

    let mut pause = false;
    let mut jump = 0.0;
    let mut run = 0.0;
    let mut landed_since_jump = false;

    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.clear();
    canvas.present();
    
    //let texture_creator = canvas.texture_creator();
    // let mut tex = texture_creator.create_texture_static(None, WINDOW_WIDTH as u32,WINDOW_HEIGHT as u32).unwrap();

    // tex.update(None, &[0u8, 0u8, 255u8, 0u8].repeat(WINDOW_WIDTH * WINDOW_HEIGHT), WINDOW_WIDTH);

   
    'running: loop {
        canvas.set_draw_color(Color::RGB(0x18, 0x18, 0x18));
        canvas.clear();
        
        let mut event_pump = sdl_context.event_pump()?;

        for event in event_pump.poll_iter() {
            match event {
                Event::Quit {..} 
                /*Event::KeyDown { keycode: Some(Keycode::Escape), .. }*/ => {
                    break 'running
                }
                
                Event::KeyDown { keycode: Some(Keycode::Up), .. } => {
                    cur_tile += 1;
                    cur_tile %= TILES.len();
                }
                Event::KeyDown { keycode: Some(Keycode::Down), .. } => {
                    if cur_tile == 0 { cur_tile = TILES.len()-1 }
                    else { cur_tile -= 1; }
                }
                // place single tile
                Event::KeyDown { keycode: Some(Keycode::Space), .. } => {
                    // grid.set(cur_x, cur_y, cur_tile, cur_size);    
                }
                Event::KeyDown { keycode: Some(Keycode::R), .. } => {
                    grid.clear();
                }
                Event::KeyDown { keycode: Some(Keycode::U), .. } => {
                    grid.update()?;
                    player.update(&grid);
                } 
                Event::KeyDown { keycode: Some(Keycode::P), .. } => {
                    pause = !pause;
                }
                Event::KeyDown { keycode: Some(Keycode::Left), .. } => {
                    if cur_size == 1 {
                        cur_size = 0;
                    }
                    cur_size += 2;
                    if cur_size >= MAX_CURSOR_SIZE { cur_size = MAX_CURSOR_SIZE; }
                }
                Event::KeyDown { keycode: Some(Keycode::Right), .. } => {
                    // FIXME: this is total horsehit idfk who came up with this (me)
                    if cur_size == 1 { cur_size = 3; }
                    cur_size -= 2;
                    if cur_size <= 0 { cur_size = 1; }
                }
                /* Event::KeyDown { keycode: Some(keycode), .. } => {
                    match keycode {
                        Keycode::Q | Keycode::A => {
                            player.move_x(-9.0);
                        }
                        Keycode::D => {
                            player.move_x(9.0);
                        }
                        _ => ()
                    }
                } */
                Event::MouseMotion { x, y, .. } => {
                    cur_x = x.clamp(0, WINDOW_WIDTH as i32) as usize / TILE_WIDTH;
                    cur_y = y.clamp(0, WINDOW_HEIGHT as i32) as usize / TILE_HEIGHT;
                }
                _ => {}
            }
        }

        let kbd = event_pump.keyboard_state();

        if kbd.is_scancode_pressed(Scancode::D) {
            player.move_x(run);
        }
        if kbd.is_scancode_pressed(Scancode::A) {
            player.move_x(-run);
        }

        if kbd.is_scancode_pressed(Scancode::D) || kbd.is_scancode_pressed(Scancode::A) {
            if run == 0.0 {
                run = 2.0;
            }
            run = run * 2.5;
            if run > PLAYER_HORIZONTAL_MOVEMENT_SPEED {
                run = PLAYER_HORIZONTAL_MOVEMENT_SPEED;
            }
        }
        else {
            run *= 0.2;
            if run < 0.001 {
                run = 0.0;
            }
        }

        if kbd.is_scancode_pressed(Scancode::Space) {
            if player.is_grounded(&grid) && landed_since_jump == true {
                jump = MAXJUMP;
                landed_since_jump = false;
                player.vel.1 -= MAXJUMP + 2.0;
            }
            player.vel.1 -= jump;
            // eprintln!("jump: {}", jump);
            jump = jump / 2.0;
            if jump < 0.001 {
                jump = 0.0;
            }
        }
        else {
            if player.is_grounded(&grid) {
                landed_since_jump = true;
            }
        }

        let left = event_pump.mouse_state().left();
        let right = event_pump.mouse_state().right();

        if left {
            grid.set(cur_x, cur_y, cur_tile, cur_size);
        }
        else if right {
            grid.set(cur_x, cur_y, 0, cur_size);
        }


        draw_cursor(cur_x, cur_y, &mut canvas, cur_size);
        grid.draw(&mut canvas);
        player.draw(&mut canvas);
        if !pause && timer % SIMULATION_FRAME_DELAY == 0 {
            grid.update()?; 
            player.update(&grid);
        }
        if let Some(rs) = grid.get_cols_in_rect(player.rect()) {
            for r in rs {
                canvas.set_draw_color(DEBUG_DRAW_COLOUR);
                //canvas.fill_rect(r);
            }
        }
        if let Some(cols) = grid.get_cols_in_rect(player.rect()) {
            for col in cols {
                let player_rect = player.rect();
                let col_obj_rect = col;

                let intersection = player_rect.intersection(col_obj_rect).unwrap();

                canvas.set_draw_color(DEBUG_DRAW_COLOUR);
                canvas.fill_rect(intersection);
            }
        }

        let (mat_texture, mat_target) = texture_and_rect_from_str(&ttf_ctx, &texture_creator, &format!("{}", TILES[cur_tile].name), DEFAULT_FONT, 24, TEXT_COLOUR);
        canvas.copy(&mat_texture, None, Some(mat_target))?;

        let (curs_tex, mut curs_targ) = texture_and_rect_from_str(&ttf_ctx, &texture_creator, &format!("Size: {}", cur_size), DEFAULT_FONT, 24, TEXT_COLOUR);
        curs_targ.x = WINDOW_WIDTH as i32-curs_targ.width() as i32;
        canvas.copy(&curs_tex, None, Some(curs_targ))?;

        let (curspos_tex, mut curspos_targ) = texture_and_rect_from_str(&ttf_ctx, &texture_creator, &format!("Pos: ({},{})", cur_x, cur_y), DEFAULT_FONT, 24, TEXT_COLOUR);
        curspos_targ.x = WINDOW_WIDTH as i32-curspos_targ.width() as i32;
        curspos_targ.y = WINDOW_HEIGHT as i32-curspos_targ.height() as i32;
        canvas.copy(&curspos_tex, None, Some(curspos_targ))?;


        canvas.present();

        timer += 1;
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / FPS));
    }
    Ok(())
}
