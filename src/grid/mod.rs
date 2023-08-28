use super::{Canvas2, TILES, WINDOW_WIDTH, WINDOW_HEIGHT};
use sdl2::rect::Rect;
use std::fmt;

mod neighbour;
pub use neighbour::*;

const G: f32 = 2.0;
const DECELERATION_Y: f32 = 1.0;
const DECELERATION_X: f32 = 0.7;
const SLOWEST_X_SPEED: f32 = 0.2;

pub const CURS_SMALLEST : usize = 1;

pub const TILE_WIDTH    : usize = 30;
pub const TILE_HEIGHT   : usize = 30;

#[derive(Debug, thiserror::Error)]
enum GridCheck {
    /// Out of bounds
    OOB,
    Obstructed,
}

impl fmt::Display for GridCheck {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "go and fuck yourself")
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Vec2(f32, f32);

impl Vec2 {
    pub const ZERO: Vec2 = Vec2(0.0, 0.0);
    const MAX_LINE_MIDPOINTS: usize = 5000;

    fn lerp(v1: &Self, v2: &Self, t: f32) -> Self {
        Vec2(lerp(v1.0, v2.0, t), lerp(v1.1, v2.1, t))
    }

    fn dist(&self, r: &Self) -> f32 {
        ((r.0 - self.0).powf(2.0) + (r.1 - self.1).powf(2.0)).sqrt() 
    }

    fn round(&self) -> (usize, usize) {
        (self.0.round() as usize, self.1.round() as usize)
    }

    fn line(p1: Vec2, p2: Vec2) -> Vec<(usize, usize)> {
        let mut pts = vec![];

        let n = ((p1.dist(&p2) as usize).clamp(1, Self::MAX_LINE_MIDPOINTS) as f32 * 1.0) as usize;
        for s in 0..n {
            let t = if n == 0 { 0.0 } else { s as f32 / n as f32 };
            pts.push(Vec2::lerp(&p1, &p2, t).round());
        }
        pts
    }
}

impl std::ops::Add<Vec2> for Vec2 {
    type Output = Vec2;
    fn add(self, rhs: Vec2) -> Self::Output {
        Self (
            self.0 + rhs.0,
            self.1 + rhs.1
        )
    }
}

impl std::ops::Mul<f32> for Vec2 {
    type Output = Vec2;
    fn mul(self, rhs: f32) -> Self::Output {
        Self (
            self.0 * rhs,
            self.1 * rhs
        )
    }
}

#[derive(Clone, Copy, Debug)]
pub enum TileIdType {
    Static,
    Dynamic,
}

#[derive(Clone, Debug)]
struct Tile {
    index: TileIndex,
    sort: TileIdType,
    vel: Vec2, 
    acc: Vec2,
    updated: bool,
}

impl Tile {
    fn new(index: TileIndex, sort: TileIdType) -> Self {
        Self {
            index, sort, vel: Vec2(0.0, 0.0), acc: Vec2(0.0, 0.0), updated: false
        }
    }
}

// FIXME: stack overflow when too many elems, move grid to the heap
pub struct Grid<const W: usize, const H: usize> {
    grid: TileGrid<W, H>,
}

type TileIndex = usize;
pub type TileGrid<const W: usize, const H: usize> = [[Tile; W]; H];

fn set_pixel<const N: usize, T>(arr: &mut [T; N], val: T, x: usize, y: usize, w: usize) {
    arr[y * w + x] = val;
}

fn lerp(start: f32, end: f32, t: f32) -> f32 {
    start * (1.0 - t) + end * t
}

impl<const W: usize, const H: usize> Grid<W, H> {

    pub fn new() -> Self {
        Grid {
            grid: std::array::from_fn(|_| std::array::from_fn(|_| Tile::new(0, TILES[0].sort)))
        }
    }

    

    pub fn draw(&mut self, canvas: &mut Canvas2) {
        for y in 0..H {
            for x in 0..W {
                let rect = Rect::new(x as i32 * TILE_WIDTH as i32, y as i32 * TILE_HEIGHT as i32, TILE_WIDTH as u32, TILE_HEIGHT as u32);
                canvas.set_draw_color(if TILES.len()-1 >= self.grid[y][x].index { TILES[self.grid[y][x].index].colour } else { (255, 0, 0) });
                canvas.fill_rect(rect);
            }
        }
    }

    fn find_free(&self, x: usize, y: usize, neighbours: &[Neighbour]) -> Result<((usize, usize), bool), GridCheck> {
        if x >= W-1 || y >= H-1 { return Err(GridCheck::OOB.into()); }

        for n in neighbours {
            match n.check_free(&self.grid, x, y) {
                Ok(r) => {
                    if n.components().contains(&Neighbour::Left) || n.components().contains(&Neighbour::Right) {
                        return Ok((r, true));
                    }
                    else { return Ok((r, false)); }
                }
                Err(_) => (),
            }
            //if let Some(r) = n.check_free(&self.grid, x, y) { return Ok(r) };
        }

        Err(GridCheck::Obstructed.into())
    }
    
    fn find_obstacle_between(&self, x1: usize, y1: usize, x2: usize, y2: usize) -> Result<(usize, usize), GridCheck> {
        todo!()
    }

    fn is_free(&self, x: usize, y: usize) -> Option<bool> {
        Some(!TILES[self.grid[if y < H { y } else { return None }][if x < W { x } else { return None }].index].solid)
    }

    fn swap(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let prev = self.grid[y2][x2].clone();
        self.grid[y2][x2] = self.grid[y1][x1].clone();
        self.grid[y1][x1] = prev;
    }

    pub fn update(&mut self) {
        let mut dyn_updates = 0usize;
        for mut x in 0..W {
            for mut y in (0..H).rev() {
                if self.grid[y][x].updated == true { 
                    self.grid[y][x].updated = false;
                    continue; 
                }
                let tile_id = &TILES[self.grid[y][x].index];
                if tile_id.gravity {
                    match tile_id.sort {
                        TileIdType::Static => {
                            if let Ok(((nx, ny), _)) = self.find_free(x, y, tile_id.neighbours) {
                                self.swap(x, y, nx, ny);
                                self.grid[ny][nx].updated = true;
                            }
                            else if let Err(GridCheck::OOB) = self.find_free(x, y, tile_id.neighbours) {
                                self.set(x, y, 0, CURS_SMALLEST);
                            }
                            else {} // do nothing, the block won't move by default
                        }
                        TileIdType::Dynamic => {
                            if let Ok(((nx, ny), _)) = self.find_free(x, y, tile_id.neighbours) {
                                // dont update if the new pos has a diff x but no x vel or yvel
                                if !(nx != x && (self.grid[y][x].vel.0 == 0.0 && self.grid[y][x].vel.1 == 0.0)) {
                                    self.update_dynamic_tile(x, y, tile_id);
                                    dyn_updates += 1;
                                }
                            }
                            // FIXME: Smoke is **incredibly** slow
                            /*let tile = &mut self.grid[y][x];
                            tile.acc.1 = G * TILES[tile.index].weight;
                            tile.vel.0 *= 0.7;
                            tile.vel.1 *= 0.7;
                            if tile.vel.0 < 0.0 && tile.vel.0 > -0.2 { tile.vel.0 = 0.0; }
                            // FIXME: tiles fly off infinitely but only to the left :,)
                            tile.vel = tile.vel + tile.acc;
                            eprintln!("INFO: tile: {:?}", tile);
                            let posv2 = Vec2(x as f32, y as f32);
                            let target = posv2 + tile.vel;
                            'inner_step: for (ptx, pty) in self.line(posv2, target) {
                                if let Err(GridCheck::OOB) = self.find_free(ptx as usize, pty as usize, [&tile_id.neighbours[..], &[Neighbour::Ident]].concat().as_slice()) {
                                    self.set(x, y, 0, CURS_SMALLEST);
                                    break 'inner_step;
                                }
                                else if let Err(GridCheck::Obstructed) = self.find_free(ptx as usize, pty as usize, [&tile_id.neighbours[..], &[Neighbour::Ident]].concat().as_slice()) {
                                    // FIXME: i think it may find obstructed when no obstructed
                                    // exists
                                    //eprintln!("INFO: obstructed");
                                    let tile = &mut self.grid[y][x];
                                    // tile.acc = Vec2::ZERO;
                                    // tile.vel = Vec2::ZERO;
                                    break 'inner_step;
                                }
                                else if let Ok((nx, ny)) = self.find_free(ptx as usize, pty as usize, [&[Neighbour::Ident], &tile_id.neighbours[..]].concat().as_slice()) {
                                    let tile = &mut self.grid[y][x];
                                    let hit_smth = tile.vel.0 as usize == 0;
                                    if nx != x && hit_smth && nx > x {
                                        tile.vel.0 = tile.vel.1/2.0;
                                        tile.vel.1 = tile.vel.1/2.0;
                                    }
                                    else if nx != x && hit_smth && nx < x {
                                        tile.vel.0 = -tile.vel.1/2.0;
                                        tile.vel.1 = tile.vel.1/2.0;
                                    }
                                    self.swap(x, y, nx as usize, ny as usize);
                                    y = ny; x = nx;
                                }
                                else {
                                    eprintln!("Uh oh");
                                }
                            }*/
                        }
                    }
                }
            }
        }
        eprintln!("dyn updates: {dyn_updates}");
    }

    fn update_dynamic_tile(&mut self, x: usize, y: usize, tile_id: &crate::TileId) {
        let tile = &mut self.grid[y][x];

        // Apply acceleration to velocity
        tile.acc = Vec2(0.0, G * tile_id.weight);
        tile.vel = tile.vel + tile.acc;

        // Apply (air resistance?) deceleration
        tile.vel.1 *= DECELERATION_Y;
        tile.vel.0 *= DECELERATION_X;
        if tile.vel.0.abs() < SLOWEST_X_SPEED { tile.vel.0 = 0.0; }

        //eprintln!("INFO: Tile: {:?}", tile);
        
        let origin = Vec2(x as f32, y as f32);
        let target = origin + tile.vel;

        if target == origin { return; }

        let mut nx = x; let mut ny = y;
        let tot_steps = Vec2::line(origin, target).iter().count();
        //eprintln!("INFO: target: {:?}, origin: {:?}, tot: {tot_steps}", target, origin);
        for (i, (ptx, pty)) in Vec2::line(origin, target).into_iter().enumerate() {
            if i == 0 { continue; }
            match self.find_free(ptx, pty, &[Neighbour::Ident]) {
                Ok(((f,g), _)) => { nx = f; ny = g; },

                Err(GridCheck::OOB) => {
                    self.set(x, y, 0, CURS_SMALLEST);
                    return;
                }
                Err(GridCheck::Obstructed) => {
                    /*if i != 0 {
                        let t = (i-1) as f32 / tot_steps as f32;
                        let collision_pt = Vec2::lerp(&origin, &target, t);

                        let tile = &mut self.grid[y][x];
                        tile.vel = Vec2::ZERO;
                        self.swap(x, y, collision_pt.0 as usize, collision_pt.1 as usize);
                        break;
                    }*/

                    if self.grid[y][x].vel.1 != 0.0 {
                        if nx != 0 && !self.get_solidity(nx - 1, ny+1) {
                            let tile = &mut self.grid[y][x];
                            tile.vel.0 = -tile.vel.1/2.0;
                        }
                        else if !self.get_solidity(nx + 1, ny+1) {
                            let tile = &mut self.grid[y][x];
                            tile.vel.0 = tile.vel.1/2.0;
                        }
                        let tile = &mut self.grid[y][x];
                        tile.vel.1 = 0.0;
                    }

                    break;
                }
            }
        }
        self.swap(x, y, nx, ny);
    }

    pub fn get_solidity(&self, x: usize, y: usize) -> bool {
        TILES[self.grid[y][x].index].solid  
    }

    pub fn set(&mut self, mut x: usize, mut y: usize, tile: TileIndex, size: usize) {
        if size == 1 {
            self.grid[y.clamp(0, H)][x.clamp(0, W)] = Tile::new( tile, TILES[tile].sort);
            return;
        }
        if x.checked_sub(size/2).is_none() { x = size/2; }
        if y.checked_sub(size/2).is_none() { y = size/2; }
        for y in y-size/2..y+size/2 {
            for x in x-size/2..x+size/2 {
                self.grid[y.clamp(0, H-1)][x.clamp(0, W-1)] = Tile::new( tile, TILES[tile].sort);
            }
        }
    }

    pub fn clear(&mut self) {
        for y in 0..H {
            for x in 0..W {
                self.grid[y][x] = Tile::new(0, TILES[0].sort);
            }
        }
    }
}
