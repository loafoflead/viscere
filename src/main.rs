use sdl2::pixels::Color;
use sdl2::event::Event;
use sdl2::keyboard::Scancode;
use sdl2::event::WindowEvent;
use sdl2::keyboard::Keycode;
use sdl2::render::TextureQuery;
use sdl2::rect::Rect;

use std::time::Duration;

mod grid;
use grid::{Grid, TILE_HEIGHT, TILE_WIDTH, neighbour::*, TileIdType};
mod vec2;
use vec2::*;
mod player;
use player::*;

const WINDOW_WIDTH      : usize = 800;
const WINDOW_HEIGHT     : usize = 600;

const CURSOR_COLOUR     : Color = Color::RGB(200, 200, 200);
const MAX_CURSOR_SIZE   : usize = 10;

const TEXT_COLOUR       : Color = Color::RGBA(255, 255, 255, 255);
const DEBUG_DRAW_COLOUR : Color = Color::RGBA(255, 0, 0, 255);
const DEFAULT_FONT      : &str  = "/usr/share/fonts/truetype/lato/Lato-Medium.ttf";


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
    sort        : TileIdType::Static,
    neighbours  : &[],
};

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
            solid: false,
            sort: TileIdType::Dynamic,
            neighbours: &[Up, UpLeft, UpRight, Left, Right],
            ..TileId::default()
        },
        TileId {
            name: "Water",
            colour: (0, 0, 255),
            gravity: true,
            solid: false,
            neighbours: &[Down, DownLeft, DownRight, Left, Right],
            ..TileId::default()
        }
    ]
};

pub struct Canvas2{
    pub canvas: sdl2::render::Canvas<sdl2::video::Window>,
    pub w: usize, pub h: usize,
}

impl Canvas2 {
    pub fn fill_rect(&mut self, rect: Rect) -> Result<(), String> {
        // let nrect = Rect::new(
        //     rect.x, rect.y,
        //     ((rect.width() as f32 / WINDOW_WIDTH as f32) * self.w as f32) as u32, 
        //     ((rect.height() as f32 / WINDOW_HEIGHT as f32) * self.h as f32) as u32,
        // );

        let nrect = self.scale_rect(rect);

        self.canvas.fill_rect(nrect)
    }

    fn scale_rect(&self, rect: Rect) -> Rect {
        Rect::new(
            ((rect.x as f64 / WINDOW_WIDTH as f64) * self.w as f64) as i32, 
            ((rect.y as f64 / WINDOW_HEIGHT as f64) * self.h as f64) as i32,
            // rect.width(), rect.height()
            ((rect.w as f64 / WINDOW_WIDTH as f64) * self.w as f64) as u32,
            ((rect.h as f64 / WINDOW_HEIGHT as f64) * self.h as f64) as u32,
        )
    }

    pub fn get_rel_wh(&self) -> (usize, usize) {
        (((TILE_WIDTH as f64 / WINDOW_WIDTH as f64) * self.w as f64) as usize,
        ((TILE_HEIGHT as f64 / WINDOW_HEIGHT as f64) * self.h as f64) as usize)
    }

    pub fn set_draw_color(&mut self, colour: Color) {
        self.canvas.set_draw_color(colour);
    }

    pub fn size(&self) -> (usize, usize) {
        (self.w, self.h)
    }

    pub fn inner(&mut self) -> &mut sdl2::render::Canvas<sdl2::video::Window> {
        &mut self.canvas
    }
}

fn draw_cursor(mut x: usize, mut y: usize, canvas: &mut Canvas2, size: usize) {
    // let rect = Rect::new(x as i32 / TILE_WIDTH as i32 * TILE_WIDTH as i32, y as i32 % TILE_HEIGHT as i32, TILE_WIDTH as u32, TILE_WIDTH as u32);
    let rect = if size == 1 {
        Rect::new(x as i32 / TILE_WIDTH as i32, y as i32 / TILE_HEIGHT as i32, TILE_WIDTH as u32, TILE_HEIGHT as u32)
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


pub fn main() -> std::result::Result<(), Box<dyn std::error::Error>> {
    let sdl_context = sdl2::init()?;
    let video_subsystem = sdl_context.video()?;
    let ttf_ctx = sdl2::ttf::init().map_err(|e| e.to_string())?;

    let window = video_subsystem.window(":(", WINDOW_WIDTH as u32, WINDOW_HEIGHT as u32)
        .opengl()
        .position_centered()
        .build()?;

    let (w, h) = (window.size().0 as usize, window.size().1 as usize);

    let canvas = window.into_canvas().build()?;
    let mut canvas = Canvas2 { canvas, w, h };

    let texture_creator = canvas.inner().texture_creator();

    // let (texture, target) = texture_and_rect_from_str(&ttf_ctx, &texture_creator, "hello world", DEFAULT_FONT, 24, TEXT_COLOUR);
        
    let mut grid = Grid::new(WINDOW_WIDTH / TILE_WIDTH, WINDOW_HEIGHT / TILE_HEIGHT)?;
    let mut timer = 0usize;

    // let mut grid: [[TileIndex; width / TILE_WIDTH]; height / TILE_HEIGHT] = ;
    let mut cur_x = 0;
    let mut cur_y = 0;
    let mut cur_tile = 1;
    let mut cur_size = 2;

    let mut player = 
        Player::new(WINDOW_WIDTH as f32 / 2.0 + 5.0, WINDOW_HEIGHT as f32 / 2.0);

    let mut pause = false;
    let mut jump = 0.0;
    let mut run = 0.0;
    let mut landed_since_jump = false;

    canvas.set_draw_color(Color::RGB(0, 255, 255));
    canvas.inner().clear();
    canvas.inner().present();
    
    //let texture_creator = canvas.texture_creator();
    // let mut tex = texture_creator.create_texture_static(None, width as u32,height as u32).unwrap();

    // tex.update(None, &[0u8, 0u8, 255u8, 0u8].repeat(width * height), width);

   
    'running: loop {
        canvas.inner().set_draw_color(Color::RGB(0x18, 0x18, 0x18));
        canvas.inner().clear();
        
        let mut event_pump = sdl_context.event_pump()?;

        let (tile_width, tile_height) = canvas.get_rel_wh();

        for event in event_pump.poll_iter() {
            match event {
                Event::Window { win_event: WindowEvent::Resized(w, h), .. } => {
                    canvas.w = w as usize;
                    canvas.h = h as usize;
                    // TODO: gotta make some sorta normalisation for all draw calls which
                    // automagically puts them in the right screen resolution innit
                }

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
                    player.pos = Vec2(WINDOW_WIDTH as f32 / 2.0, WINDOW_HEIGHT as f32 / 2.0);
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
                    cur_x = (x as f32 / canvas.w as f32 * WINDOW_WIDTH as f32) as usize / TILE_WIDTH;
                    cur_y = (y as f32 / canvas.h as f32 * WINDOW_HEIGHT as f32) as usize / TILE_HEIGHT;
                    // cur_x = x.clamp(0, canvas.w as i32) as usize;
                    // cur_y = y.clamp(0, canvas.h as i32) as usize;
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
                player.move_y(-(MAXJUMP + 2.0));
            }
            player.move_y(-jump);
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

        // don't crash if we fail to place a tile, it doesn't really matter
        // TODO: make this log instead of crash
        if left {
            let _ = grid.set(cur_x, cur_y, cur_tile, cur_size);
        }
        else if right {
            let _ = grid.set(cur_x, cur_y, 0, cur_size);
        }


        grid.draw(&mut canvas);
        player.draw(&mut canvas)?;
        draw_cursor(cur_x, cur_y, &mut canvas, cur_size);
        if !pause && timer % SIMULATION_FRAME_DELAY == 0 {
            grid.update()?; 
            player.update(&grid);
        }
        
        let (width, height) = canvas.size();
        
        let canvas = canvas.inner();

        let (mat_texture, mat_target) = texture_and_rect_from_str(&ttf_ctx, &texture_creator, &format!("{}", TILES[cur_tile].name), DEFAULT_FONT, 24, TEXT_COLOUR);
        canvas.copy(&mat_texture, None, Some(mat_target))?;

        let (curs_tex, mut curs_targ) = texture_and_rect_from_str(&ttf_ctx, &texture_creator, &format!("Size: {}", cur_size), DEFAULT_FONT, 24, TEXT_COLOUR);
        curs_targ.x = width as i32-curs_targ.width() as i32;
        canvas.copy(&curs_tex, None, Some(curs_targ))?;

        let (curspos_tex, mut curspos_targ) = texture_and_rect_from_str(&ttf_ctx, &texture_creator, &format!("Pos: ({},{})", cur_x, cur_y), DEFAULT_FONT, 24, TEXT_COLOUR);
        curspos_targ.x = width as i32-curspos_targ.width() as i32;
        curspos_targ.y = height as i32-curspos_targ.height() as i32;
        canvas.copy(&curspos_tex, None, Some(curspos_targ))?;


        canvas.present();

        timer += 1;
        ::std::thread::sleep(Duration::new(0, 1_000_000_000u32 / FPS));
    }
    Ok(())
}
