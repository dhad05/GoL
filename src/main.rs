extern crate gtk;
extern crate gdk;
extern crate gobject_sys;
extern crate cairo;
extern crate rand;

use gtk::prelude::*;
use gtk::{Window, WindowType, DrawingArea, Button};
use std::thread;
use rand::Rng;
use std::sync::{Arc, Mutex};

const WIDTH: i32 = 200;
const HEIGHT: i32 = 200;
const CELL_SIZE: i32 = 3;

type Map = [[u32; HEIGHT as usize]; WIDTH as usize];

struct Area(DrawingArea);

unsafe impl Send for Area {
}

fn zerotable() -> Map {
    [[0; HEIGHT as usize]; WIDTH as usize]
}
fn randomtable() -> Map {
    let mut r = rand::thread_rng();
    let mut ret = zerotable();
    for i in 0..WIDTH as usize {
        for j in 0..HEIGHT as usize {
            ret[i][j] = r.gen::<u32>() & 1;
        }
    }
    ret   
}

fn update(map: &mut Map) {
    let mut new = zerotable();
    {
        let get = |x: i32, y: i32| -> Option<u32> {
            if x<0 || x>=WIDTH || y<0 || y>=HEIGHT {
                return None;
            }
            Some(map[x as usize][y as usize])
        };
        for i in 0..WIDTH { for j in 0..HEIGHT {
            for x in i-1..i+2 { for y in j-1..j+2 { 
                match get(x, y) {
                    Some(u) => new[i as usize][j as usize] += u,
                    None => (),
                }
            }}
            new[i as usize][j as usize] -= get(i, j).unwrap();
        }}
    }
   for i in 0..WIDTH as usize { for j in 0..HEIGHT as usize {
        if map[i][j] == 1 {
            if new[i][j] < 2 || new[i][j] > 3 {
                map[i][j] = 0
            }
        } else if new[i][j] == 3 {
            map[i][j] = 1;
        }
    }}
}

fn main() {
    if gtk::init().is_err() {
        println!("failed!!");    
    }
    let window = Window::new(WindowType::Toplevel);
    let vbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let area = Area(DrawingArea::new());
    area.0.set_size_request(WIDTH*CELL_SIZE, HEIGHT*CELL_SIZE);
    window.add(&vbox);
    vbox.pack_start(&area.0, false, false, 0);
    window.set_title("Game of Life");
    window.show_all();
    
    let map = Arc::new(Mutex::new(randomtable()));
    let mapdata = map.clone();
    area.0.connect_draw(move |_, cr| {
        let mut map = mapdata.lock().unwrap();
        cr.scale(CELL_SIZE as f64, CELL_SIZE as f64);
        cr.set_source_rgb(1f64, 1f64, 1f64);
        cr.paint();
        cr.set_source_rgb(0f64, 0f64, 0f64);
        update(&mut map);
        for i in 0..WIDTH as usize {
            for j in 0..HEIGHT as usize {
                if map[i][j] == 1 {
                    cr.rectangle(i as f64, j as f64, 1.0, 1.0);
                }
            }
        }
        cr.fill();
        Inhibit(true)
    });
 
    let loop_thread = thread::spawn( move || {
        let duration = std::time::Duration::from_millis(50);
        loop {
            thread::sleep(duration);
            area.0.queue_draw();
        }
    });

    window.connect_delete_event( |_, _| {
        gtk::main_quit();
        Inhibit(true)
    });
    gtk::main();
}
