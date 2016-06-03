extern crate gtk;
extern crate gdk;
extern crate gdk_sys;
extern crate cairo;
extern crate rand;

use gtk::prelude::*;
use gtk::{Window, WindowType, DrawingArea, Button};
use std::thread;
use rand::Rng;
use std::sync::{Arc, Mutex};
use gdk::enums::modifier_type;

const WIDTH: i32 = 200;
const HEIGHT: i32 = 200;
const CELL_SIZE: i32 = 3;

type Map = [[u32; HEIGHT as usize]; WIDTH as usize];

// `MapTrait` provide a safe and convenient way to
// access `Map`
trait MapTrait {
    fn get(&self, i32, i32) -> Option<u32>;
    fn set(&mut self, i32, i32, u32);
    fn update(&mut self);
    fn copy_from(&mut self, &Map);
}
// `State` is used for handling events
struct State{
    run: bool,
    repaint: bool,
    exited: bool,
    next: bool,
}
struct Area(DrawingArea);

unsafe impl Send for Area {
}

fn zerotable() -> Map {
    [[0; HEIGHT as usize]; WIDTH as usize]
}

fn randomtable() -> Map {
    let mut r = rand::thread_rng();
    let mut ret = zerotable();
    for i in 0..WIDTH as usize { for j in 0..HEIGHT as usize {
        ret[i][j] = r.gen::<u32>() & 1;
    }}
    ret   
}

fn check(x: i32, y: i32) -> bool {
    if x<0 || x>=WIDTH || y<0 || y>=HEIGHT {
        return false;
    }
    true
}

impl MapTrait for Map {
    fn get(&self, x: i32, y: i32) -> Option<u32> {
        if check(x, y) {
            return Some(self[x as usize][y as usize]);
        }
        None
    }

    fn set(&mut self, x: i32, y: i32, val: u32) {
        if check(x, y) {
            self[x as usize][y as usize] = val;
        }
    }

    fn update(&mut self) {
        // `nei` contains the number of neighbours of each cell
        let mut nei = zerotable();
        // calculate `nei`
        for i in 0..WIDTH { for j in 0..HEIGHT {
            for x in i-1..i+2 { for y in j-1..j+2 { 
                match self.get(x, y) {
                    Some(u) => nei[i as usize][j as usize] += u,
                    None => (),
                }
            }}
            nei[i as usize][j as usize] -= self.get(i, j).unwrap();
        }}
        
        // update `self` with game of life's rules
        for i in 0..WIDTH as usize { for j in 0..HEIGHT as usize {
            if self[i][j] == 1 {
                if nei[i][j] < 2 || nei[i][j] > 3 {
                    self[i][j] = 0
                }
            } else if nei[i][j] == 3 {
                self[i][j] = 1;
            }
        }}
    }

    fn copy_from(&mut self, s: &Map) {
        for i in 0..WIDTH as usize { for j in 0..HEIGHT as usize {
            self[i][j] = s[i][j]
        }}
    }
}

fn main() {
    if gtk::init().is_err() {
        println!("failed!!");    
    }
    // these variables need to be shared between threads 
    // so wrap it into Arc(Mutex())
    // for more infos, view the `concurency` section on rust doc
    let map = Arc::new(Mutex::new(zerotable()));
    let state = Arc::new(Mutex::new(State{
        run: false,
        repaint: false,
        exited: false,
        next: false,
    }));
    let cell_size = Arc::new(Mutex::new(CELL_SIZE));

    let window = Window::new(WindowType::Toplevel);
    let hbox = gtk::Box::new(gtk::Orientation::Horizontal, 0);
    let button_box = gtk::ButtonBox::new(gtk::Orientation::Vertical);
    let pause_button = Button::new_with_label(
        if state.lock().unwrap().run { "Pause" }
        else { "Start" }
    );
    let random_button = Button::new_with_label("Randomize");
    let next_button = Button::new_with_label("Next");
    let clear_button = Button::new_with_label("Clear");
    let zoom_in_button = Button::new_with_label("Zoom in");
    let zoom_out_button = Button::new_with_label("Zoom out");
    let area = Area(DrawingArea::new());
    area.0.set_size_request(WIDTH*CELL_SIZE, HEIGHT*CELL_SIZE);

    /* Ask to recieve events the drawing area doesn't normally
     * subscribe to
     * */
    area.0.set_events(area.0.get_events() | (
                      gdk_sys::GDK_POINTER_MOTION_MASK |
                      gdk_sys::GDK_BUTTON_PRESS_MASK
                      ).bits() as i32);

    // ask button_box to place widgets from top
    button_box.set_layout(gtk::ButtonBoxStyle::Start);
    button_box.pack_start(&pause_button, false, false, 0);
    button_box.pack_start(&next_button, false, false, 0);
    button_box.pack_start(&random_button, false, false, 0);
    button_box.pack_start(&clear_button, false, false, 0);
    button_box.pack_start(&zoom_in_button, false, false, 0);
    button_box.pack_start(&zoom_out_button, false, false, 0);
    hbox.pack_start(&area.0, false, false, 0);
    hbox.pack_start(&button_box, false, false, 0);
    window.add(&hbox);
    window.set_title("Game of Life");
    window.show_all();

    {// connect draw function
     // also update the map each time we re-draw
        let map = map.clone();
        let state = state.clone();
        let cell_size = cell_size.clone();
        area.0.connect_draw( move |_, cr| {
            let mut map = map.lock().unwrap();
            {
                let mut state = state.lock().unwrap();
                if state.run {
                    map.update();
                }else if state.next {
                    map.update();
                    state.next = false;
                }
            }
            (|x: f64| cr.scale(x, x)) (*cell_size.lock().unwrap() as f64);
            cr.set_source_rgb(1f64, 1f64, 1f64);
            cr.paint();
            cr.set_source_rgb(0f64, 0f64, 0f64);
            for i in 0..WIDTH as usize { for j in 0..HEIGHT as usize {
                if map[i][j] == 1 {
                    cr.rectangle(i as f64, j as f64, 1.0, 1.0);
                }
            }}
            cr.fill();
            Inhibit(true)
        });
    }

    {// mouse painting event
        let map = map.clone();
        let state = state.clone();
        let cell_size = cell_size.clone();
        area.0.connect_motion_notify_event( move |_, ev| {
            let set = |val: u32| {
                let size = *cell_size.lock().unwrap();
                let mapfn = |x: f64| (x as i32 / size );
                let (x, y) = ev.get_position();
                let (x, y) = (mapfn(x), mapfn(y));
                map.lock().unwrap().set(x, y, val);
                state.lock().unwrap().repaint = true;
            };
            let ev_state = ev.get_state();
            if (ev_state & modifier_type::Button1Mask).bits() != 0 {
                // if left mouse is clicked
                set(1);
            }else if (ev_state & modifier_type::Button3Mask).bits() != 0 {
                // if right mouse is clicked
                set(0);
            }
            Inhibit(true)
        });
    }

    {// mouse click event
        let map = map.clone();
        let state = state.clone();
        let cell_size = cell_size.clone();
        area.0.connect_button_press_event( move |_, ev| {
            let set = |val: u32| {
                let size = *cell_size.lock().unwrap();
                let mapfn = |x: f64| (x as i32 / size );
                let (x, y) = ev.get_position();
                let (x, y) = (mapfn(x), mapfn(y));
                map.lock().unwrap().set(x, y, val);
                state.lock().unwrap().repaint = true;
            };
            // get button keyval
            let button = ev.as_ref().button as i32;
            if button == gdk_sys::GDK_BUTTON_PRIMARY {
                set(1);
            }else if button == gdk_sys::GDK_BUTTON_SECONDARY {
                set(0);
            }
            Inhibit(true)
        });
    }

    {// next_button connect
        let state = state.clone();
        next_button.connect_clicked( move |_| {
            let mut state = state.lock().unwrap();
            if !state.run {
                state.next = true;
                state.repaint = true;
            } else {
                state.next = false;
            }
        });
    }

    {// zoom in binding
        let state = state.clone();
        let cell_size = cell_size.clone();
        zoom_in_button.connect_clicked( move |_| {
            *cell_size.lock().unwrap() += 1;
            state.lock().unwrap().repaint = true;
        });
    }

    {// zoom out binding
        let state = state.clone();
        let cell_size = cell_size.clone();
        zoom_out_button.connect_clicked( move |_| {
            let mut x = cell_size.lock().unwrap();
            if *x > CELL_SIZE {
                *x -= 1;
            }
            state.lock().unwrap().repaint = true;
        });
    }

    
    {// pause_button binding
        let state = state.clone();
        pause_button.connect_clicked( move |button| {
            let mut state = state.lock().unwrap();
            state.run = !state.run;
            button.set_label(
                if state.run { "Pause" }
                else { "Start" }
            );
        });
    }

    {// clear_button binding
        let map = map.clone();
        let state = state.clone();
        clear_button.connect_clicked( move |_| {
            map.lock().unwrap().copy_from(&zerotable());
            state.lock().unwrap().repaint = true;
        });
    }

    {// random_button binding
        let map = map.clone();
        let state = state.clone();
        random_button.connect_clicked( move |_| {
            map.lock().unwrap().copy_from(&randomtable());
            state.lock().unwrap().repaint = true;
        });
    }

    // main loop thread
    let loop_thread = {
        let state = state.clone();
        thread::spawn( move || {
            let duration = std::time::Duration::from_millis(50);
            loop {
                thread::sleep(duration);
                let mut state = state.lock().unwrap();
                if state.exited {
                    break;
                }
                if state.run {
                    // `queue_draw` will ask gtk to 
                    // repaint the widget
                    area.0.queue_draw();
                }else if state.repaint {
                    state.repaint = false;
                    area.0.queue_draw();
                }
            }
        })
    };

    {
        let state = state.clone();
        window.connect_delete_event( move |_, _| {
            let mut state = state.lock().unwrap();
            state.exited = true;
            gtk::main_quit();
            Inhibit(true)
        });
    }
    gtk::main();
    // wait for the thread to stop
    match loop_thread.join() {
        Err(_) => println!("Some error ocured"),
        Ok(_) => (),
    }
}

