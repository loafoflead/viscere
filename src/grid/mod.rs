use super::{Canvas2, TILES, WINDOW_HEIGHT, WINDOW_WIDTH};
use sdl2::rect::Rect;
use std::fmt;

pub mod neighbour;
pub use neighbour::*;

pub const CURS_SMALLEST : usize = 1;

pub const TILE_WIDTH    : usize = 10;
pub const TILE_HEIGHT   : usize = 10;

pub const MAX_GRID_WIDTH: usize = 800;
pub const MAX_GRID_HEIGHT: usize = 600;

pub type Result<T> = ::std::result::Result<T, GridResult>;

#[derive(Debug, thiserror::Error)]
pub enum GridResult {
    GridTooLarge,
    OOB,
    Obstructed,
}

impl fmt::Display for GridResult {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "go and fuck yourself")
    }
}



#[derive(Clone, Copy, Debug)]
pub enum TileIdType {
    Static,
    Dynamic,
}

#[derive(Copy, Clone, Debug)]
pub struct Tile {
    index: TileIndex,
    updated: bool,
}

impl Tile {
    fn new(index: TileIndex) -> Self {
        Self {
            index, updated: false
        }
    }
}

impl Default for Tile {
    fn default() -> Self {
        Tile {
            index: 0, 
            updated: false
        }
    }
}

// FIXME: stack overflow when too many elems, move grid to the heap
pub struct Grid {
    grid: Vec<Tile>,
    width: usize,
    height: usize,
}

type TileIndex = usize;

impl Grid { 
    pub fn new(w: usize, h: usize) -> Result<Self> {
        Grid::assert_wh(w, h)?;
        Ok(Grid {
            grid: [Tile::default()].repeat(w*h),
            width: w, height: h
        })
    }

    fn assert_wh(w: usize, h: usize) -> Result<()> {
        if w >= MAX_GRID_WIDTH || h >= MAX_GRID_HEIGHT {
            Err(GridResult::GridTooLarge)
        }
        else { Ok(()) }
    }

    fn assert_inbounds(&self, x: isize, y: isize) -> Result<()> {
        if x >= 0 && y >= 0 && (x.abs() as usize) < self.width && (y.abs()as usize) < self.height {
            Ok(())
        }
        else {
            Err(GridResult::OOB.into())
        }
    }

    pub(self) fn get_wh(&self) -> (usize, usize) {
        (self.width, self.height)
    }

    pub fn draw(&mut self, canvas: &mut Canvas2) {
        let (w, h) = self.get_wh();

        for y in 0..h {
            for x in 0..w {
                let rect = Rect::new(x as i32 * TILE_WIDTH as i32, y as i32 * TILE_HEIGHT as i32, TILE_WIDTH as u32, TILE_HEIGHT as u32);
                canvas.set_draw_color(if TILES.len()-1 >= self[(x, y)].index { TILES[self[(x, y)].index].colour.into() } else { (255, 0, 0).into() });
                let _ = canvas.fill_rect(rect);
            }
        }
    }

    pub fn get_cols_in_rect(&self, rect: Rect) -> Option<Vec<Rect>> {
        let (w, h) = self.get_wh();
        if rect.x < 0 || rect.y < 0 || rect.x as usize / TILE_WIDTH >= w || rect.y as usize / TILE_HEIGHT >= h { return None; }

        let x_range = rect.x as usize / TILE_WIDTH..(rect.x as usize + rect.w as usize + TILE_WIDTH - 1) / TILE_WIDTH;
        let y_range = rect.y as usize / TILE_HEIGHT..(rect.y as usize + rect.h as usize + TILE_HEIGHT - 1) / TILE_HEIGHT;

        let mut res = vec!();

        for y in y_range {
            for x in x_range.clone() {
                self.assert_inbounds(x as isize, y as isize).ok()?;
                if TILES[self[(x, y)].index].solid {
                    let rect = Rect::new(
                        x as i32 * TILE_WIDTH as i32, 
                        y as i32 * TILE_HEIGHT as i32, 
                        TILE_WIDTH as u32,
                        TILE_HEIGHT as u32,
                    );
                    res.push(rect);
                }
            }
        }

        Some(res)
    }

    fn find_free(&self, x: usize, y: usize, neighbours: &[Neighbour]) -> Result<(usize, usize)> {
        let (w, h) = self.get_wh();
        if x >= w || y >= h { return Err(GridResult::OOB.into()); }

        if neighbours == &[Neighbour::Ident] {
            return neighbours[0].check_free(&self, x, y);
        }
        
        let mut oob = true;
        for n in neighbours {
            match n.check_free(&self, x, y) {
                Ok(r) => {
                    if n.components().contains(&Neighbour::Left) || n.components().contains(&Neighbour::Right) {
                        return Ok(r);
                    }
                    else { return Ok(r); }
                }
                Err(GridResult::OOB) => {
                    return Err(GridResult::OOB);
                    // oob &= true;
                }
                _ => {
                    oob &= false;
                }
            }
            //if let Some(r) = n.check_free(&self.grid, x, y) { return Ok(r) };
        }

        if oob == true {
            Err(GridResult::OOB)
        }
        else {
            Err(GridResult::Obstructed.into())
        }
    }

    fn swap(&mut self, x1: usize, y1: usize, x2: usize, y2: usize) {
        let prev = self[(x2, y2)].clone();
        self[(x2, y2)] = self[(x1, y1)].clone();
        self[(x1, y1)] = prev;
    }

    pub fn update(&mut self) -> Result<()> {
        for t in &mut self.grid {
            t.updated = false;
        }
        let (w, h) = self.get_wh();
        for y in 0..h {
            for x in 0..w {
                let tile_id = &TILES[self[(x, y)].index];
                if tile_id.gravity && !self[(x, y)].updated {
                    self.update_static_tile(x, y, &tile_id)?;
                }
            }
        }
        Ok(())
    }

    fn update_static_tile(&mut self, x: usize, y: usize, tile_id: &crate::TileId) -> Result<()> {
        if let Ok((nx, ny)) = self.find_free(x, y, tile_id.neighbours) {
            self.swap(x, y, nx, ny);
            self[(nx, ny)].updated = true;
        }
        else if let Err(GridResult::OOB) = self.find_free(x, y, tile_id.neighbours) {
            self.set(x, y, 0, CURS_SMALLEST)?;
        }
        else {} // do nothing, the block won't move by default
        Ok(())
    }

    pub fn set(&mut self, mut x: usize, mut y: usize, tile: TileIndex, size: usize) -> Result<()> {
        self.assert_inbounds(x as isize, y as isize)?;
        let (w, h) = self.get_wh();
        if size == 1 {
            self[(x, y)] = Tile::new(tile);
            return Ok(());
        }

        if x.checked_sub(size/2).is_none() { x = size/2; }
        if y.checked_sub(size/2).is_none() { y = size/2; }

        for y in y-size/2..y+size/2 {
            for x in x-size/2..x+size/2 {
                self[(x.clamp(0, w-1), y.clamp(0, h-1))] = Tile::new( tile);
           }
        }
        Ok(())
    }

    pub fn clear(&mut self) {
        for e in &mut self.grid {
            *e = Tile::default();
        }
    }
}

use std::ops::{Index, IndexMut};

impl Index<(usize, usize)> for Grid {
    type Output = Tile;

    fn index(&self, (x, y): (usize, usize)) -> &Self::Output {
        &self.grid[y * self.width + x]
    }
}

impl IndexMut<(usize, usize)> for Grid {
    fn index_mut(&mut self, (x, y): (usize, usize)) -> &mut Self::Output {
        &mut self.grid[y * self.width + x]
    }
}
