// Copyright 2014 The Servo Project Developers. See the COPYRIGHT
// file at the top-level directory of this distribution.
//
// Licensed under the Apache License, Version 2.0 <LICENSE-APACHE or
// http://www.apache.org/licenses/LICENSE-2.0> or the MIT license
// <LICENSE-MIT or http://opensource.org/licenses/MIT>, at your
// option. This file may not be copied, modified, or distributed
// except according to those terms.

#![crate_type = "lib"]
#![crate_name = "efl"]
#![license = "ASL2/MIT"]
#![comment = "Servo-specific wrapper for the relevant components of the \
              Enlightenment Foundation Libraries."]

#![feature(globs)]
#![feature(macro_rules)]
#![feature(unsafe_destructor)]

extern crate libc;
extern crate sync;

use std::mem;
use std::ptr;
use std::str;

pub mod ffi;

#[deriving(Clone)]
pub struct Context {
    supported_engines: Vec<Engine>,
}

#[deriving(Show)]
pub enum InitError {
    EcoreInitError,
    EcoreEvasInitError,
    AlreadyInitialized,
}

pub fn init() -> Result<Context, InitError> {
    use sync::one::{Once, ONCE_INIT};

    static mut INIT: Once = ONCE_INIT;
    let mut result = Err(AlreadyInitialized);
    unsafe {
        INIT.doit(|| {
            result = if ffi::ecore_init() == 0 {
                Err(EcoreInitError)
            } else if ffi::ecore_evas_init() == 0 {
                // Evas was not initialised, but we still need to shut down
                // ecore on exiting
                std::rt::at_exit(proc() {
                    ffi::ecore_shutdown();
                });
                Err(EcoreEvasInitError)
            } else {
                // Get a list of the supported engines
                let engines_ptr = ffi::ecore_evas_engines_get();
                let engines = ffi::eina_list_iter(engines_ptr as *const _).map(|data| {
                    Engine::parse(str::raw::from_c_str(data as *const _))
                }).collect();
                ffi::ecore_evas_engines_free(engines_ptr);

                // We will need to shut down evas on exiting
                std::rt::at_exit(proc() {
                    ffi::ecore_evas_shutdown();
                    ffi::ecore_shutdown();
                });

                Ok(Context { supported_engines: engines, })
            };
        });
    }
    result
}

// Generates an enum that specifies the possible engines that EFL can use.
macro_rules! engines {
    ($($Engine:ident => $name:pat),+) => {
        /// A rendering engine identifier
        #[deriving(Clone, Show, PartialEq, Eq)]
        pub enum Engine {
            $($Engine,)+
            Unknown(String),
        }

        impl Engine {
            fn parse(src: String) -> Engine {
                match src.as_slice() {
                    $($name => $Engine,)+
                    _ => Unknown(src),
                }
            }

            fn get_efl_name<'a>(&'a self) -> &'a str {
                match *self {
                    $($Engine => stringify!($name),)+
                    Unknown(ref src) => src.as_slice(),
                }
            }
        }
    }
}

// This engine list is taken from the implementation of `_ecore_evas_available_engines_get`
// which can be found in `efl/src/lib/ecore_evas/ecore_evas_module.c`.
engines! {
    Fb              => "fb",
    OpenGlX11       => "opengl_x11",
    SoftwareX11     => "software_x11",
    Buffer          => "buffer",
    Ews             => "ews",
    OpenglCocoa     => "opengl_cocoa",
    Psl1ght         => "psl1ght",
    OpenglSdl       => "opengl_sdl",
    Sdl             => "sdl",
    WaylandShm      => "wayland_shm",
    WaylandEgl      => "wayland_egl",
    SoftwareGdi     => "software_gdi",
    SoftwareDdraw   => "software_ddraw",
    Direct3d        => "direct3d",
    OpenGlGlew      => "opengl_glew"
}

impl Context {
    pub fn build_window<'a>(&'a self, x: i32, y: i32, w: i32, h: i32) -> WindowBuilder<'a> {
        WindowBuilder {
            context: self,
            engine: None,
            x: x, y: y,
            w: w, h: h,
            gl_config: GlConfig::new(),
        }
    }

    pub fn main_loop_begin(&self) {
        unsafe { ffi::ecore_main_loop_begin() };
    }

    pub fn main_loop_quit(&self) {
        println!("bye");
        unsafe { ffi::ecore_main_loop_quit() }
    }

    pub fn get_supported_engines<'a>(&'a self) -> &'a [Engine] {
        self.supported_engines.as_slice()
    }
}

/// Surface color format.
#[repr(u32)]
pub enum ColorFormat {
    Rgb888 = ffi::EVAS_GL_RGB_888,
    Rgba8888 = ffi::EVAS_GL_RGBA_8888,
}

/// Surface depth format.
#[repr(u32)]
pub enum DepthBits {
    DepthBits8 = ffi::EVAS_GL_DEPTH_BIT_8,
    DepthBits16 = ffi::EVAS_GL_DEPTH_BIT_16,
    DepthBits24 = ffi::EVAS_GL_DEPTH_BIT_24,
    DepthBits32 = ffi::EVAS_GL_DEPTH_BIT_32,
}
/// Surface stencil format.
#[repr(u32)]
pub enum StencilBits {
    StencilBits1 = ffi::EVAS_GL_STENCIL_BIT_1,
    StencilBits2 = ffi::EVAS_GL_STENCIL_BIT_2,
    StencilBits4 = ffi::EVAS_GL_STENCIL_BIT_4,
    StencilBits8 = ffi::EVAS_GL_STENCIL_BIT_8,
    StencilBits16 = ffi::EVAS_GL_STENCIL_BIT_16,
}

bitflags! {
    #[doc = "OpenGl configuration options"]
    flags GlOptions: libc::c_uint {
        #[doc = "No extra options."]
        static GlOptionsNone = ffi::EVAS_GL_OPTIONS_NONE,
        #[doc = "Allow rendering directly to the window when possible."]
        static GlOptionsDirect = ffi::EVAS_GL_OPTIONS_DIRECT
    }
}

/// Options for a multisample, anti-aliased (MSAA) rendering surface.
///
/// Only works for supported devices.
#[repr(u32)]
pub enum MultisampleBits {
    /// MSAA with a minimum number of samples.
    MultisampleLow = ffi::EVAS_GL_MULTISAMPLE_LOW,
    /// MSAA with half the maximum number of samples.
    MultisampleMedium = ffi::EVAS_GL_MULTISAMPLE_MED,
    /// MSAA with the maximum number of samples allowed.
    MultisampleHigh = ffi::EVAS_GL_MULTISAMPLE_HIGH,
}

/// A struct that holds the OpenGL configuration. EFL requires us to allocate
/// and deallocate the configuration instead of instantiating the struct
/// ourselves for backwards compatibility reasons.
struct GlConfig {
    ptr: *mut ffi::Evas_GL_Config,
}

impl GlConfig {
    fn new() -> GlConfig {
        let ptr = unsafe { ffi::evas_gl_config_new() };
        assert!(!ptr.is_null());
        GlConfig { ptr: ptr }
    }

    fn set_color_format(&mut self, format: ColorFormat) {
        unsafe { (*self.ptr).color_format = format as libc::c_uint };
    }

    fn set_depth(&mut self, depth: Option<DepthBits>) {
        unsafe { (*self.ptr).depth_bits = depth.map_or(ffi::EVAS_GL_DEPTH_NONE, |x| x as libc::c_uint) };
    }

    fn set_stencil(&mut self, stencil: Option<StencilBits>) {
        unsafe { (*self.ptr).stencil_bits = stencil.map_or(ffi::EVAS_GL_STENCIL_NONE, |x| x as libc::c_uint) };
    }

    fn set_options(&mut self, options: GlOptions) {
        unsafe { (*self.ptr).options_bits = options.bits };
    }

    fn set_multisample(&mut self, multisample: Option<MultisampleBits>) {
        unsafe { (*self.ptr).multisample_bits = multisample.map_or(ffi::EVAS_GL_MULTISAMPLE_NONE, |x| x as libc::c_uint) };
    }
}

impl Drop for GlConfig {
    fn drop(&mut self) {
        unsafe { ffi::evas_gl_config_free(self.ptr) };
    }
}

pub struct WindowBuilder<'a> {
    context: &'a Context,
    engine: Option<Engine>,
    x: i32, y: i32,
    w: i32, h: i32,
    gl_config: GlConfig,
}

impl<'a> WindowBuilder<'a> {
    /// Specify the rendering engine to use with the window. If this is not
    /// specified the first working rendering engine will be used.
    pub fn with_engine(mut self, engine: Engine) -> WindowBuilder<'a> {
        self.engine = Some(engine); self
    }

    pub fn with_color_format(mut self, format: ColorFormat) -> WindowBuilder<'a> {
        self.gl_config.set_color_format(format); self
    }

    pub fn with_gl_depth(mut self, depth: Option<DepthBits>) -> WindowBuilder<'a> {
        self.gl_config.set_depth(depth); self
    }

    pub fn with_gl_stencil(mut self, stencil: Option<StencilBits>) -> WindowBuilder<'a> {
        self.gl_config.set_stencil(stencil); self
    }

    pub fn with_gl_options(mut self, options: GlOptions) -> WindowBuilder<'a> {
        self.gl_config.set_options(options); self
    }

    pub fn with_gl_multisample(mut self, multisample: Option<MultisampleBits>) -> WindowBuilder<'a> {
        self.gl_config.set_multisample(multisample); self
    }

    pub fn create(self) -> Result<Window<'a>, ()> {
        let WindowBuilder {
            context,
            engine,
            x, y, w, h,
            gl_config,
        } = self;

        let ee = unsafe {
            match engine {
                Some(ref engine) => engine.get_efl_name().with_c_str(|name| {
                    ffi::ecore_evas_new(name, x, y, w, h, ptr::null())
                }),
                None => {
                    ffi::ecore_evas_new(ptr::null(), x, y, w, h, ptr::null())
                },
            }
        };
        if !ee.is_null() {
            let canvas = unsafe { ffi::ecore_evas_get(ee as *const _) };
            let object = unsafe { ffi::evas_object_image_add(canvas) };
            let window = Window {
                context: context,
                ee: ee,
                canvas: canvas,
                object: object,
                _gl_config: gl_config,
                event_callbacks: EventCallbacks::new(),
                input_callbacks: InputCallbacks::new(),
            };
            unsafe {
                ffi::evas_object_resize(window.object, w, h);
                ffi::evas_object_focus_set(window.object, ffi::EINA_TRUE);
                ffi::evas_object_show(window.object);
                // We store a pointer back to the window so that the
                // `extern "C"` event callbacks can access their corresponding
                // Rust callbacks in the `EventCallbacks` vtable.
                let window_ptr: *const Window = &window;
                Window::data_ptr_key().with_c_str(|key| {
                    ffi::ecore_evas_data_set(window.ee, key, window_ptr as *const _)
                });
            }
            Ok(window)
        } else {
            Err(())
        }
    }
}

pub struct Window<'a> {
    context: &'a Context,
    ee: *mut ffi::Ecore_Evas,
    #[allow(dead_code)]
    canvas: *mut ffi::Evas,
    object: *mut ffi::Evas_Object,
    /// Carry these parameters for the lifetime of the window, and destroy them
    /// at afterwards
    _gl_config: GlConfig,
    /// The vtable of event callbacks associated with the window
    event_callbacks: EventCallbacks,
    /// The vtable of input callbacks associated with the window
    input_callbacks: InputCallbacks,
}

impl<'a> std::fmt::Show for Window<'a> {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "Window({}, {}, {}, {})", self.ee, self.canvas, self.object, self.event_callbacks.resize.is_some())
    }
}

impl<'a> Window<'a> {
    fn data_ptr_key() -> &'static str { "WINDOW_PTR" }

    pub fn get_context<'a>(&'a self) -> &'a Context { self.context }

    pub fn set_iconified(&self, on: bool) {
        unsafe { ffi::ecore_evas_iconified_set(self.ee, ffi::to_eina_bool(on)) };
    }

    pub fn is_iconified(&self) -> bool {
        unsafe { ffi::from_eina_bool(ffi::ecore_evas_iconified_get(self.ee as *const _)) }
    }

    pub fn set_borderless(&self, on: bool) {
        unsafe { ffi::ecore_evas_borderless_set(self.ee, ffi::to_eina_bool(on)) };
    }

    pub fn is_borderless(&self) -> bool {
        unsafe { ffi::from_eina_bool(ffi::ecore_evas_borderless_get(self.ee as *const _)) }
    }

    pub fn set_fullscreen(&self, on: bool) {
        unsafe { ffi::ecore_evas_fullscreen_set(self.ee, ffi::to_eina_bool(on)) };
    }

    pub fn is_fullscreen(&self) -> bool {
        unsafe { ffi::from_eina_bool(ffi::ecore_evas_fullscreen_get(self.ee as *const _)) }
    }

    pub fn set_aspect_ratio(&self, aspect_ratio: f64) {
        unsafe { ffi::ecore_evas_aspect_set(self.ee, aspect_ratio as libc::c_double) };
    }

    pub fn get_aspect_ratio(&self) -> f64 {
        unsafe { ffi::ecore_evas_aspect_get(self.ee as *const _) as f64 }
    }

    pub fn set_demand_attention(&self, on: bool) {
        unsafe { ffi::ecore_evas_demand_attention_set(self.ee, ffi::to_eina_bool(on)) };
    }

    pub fn demands_attention(&self) -> bool {
        unsafe { ffi::from_eina_bool(ffi::ecore_evas_demand_attention_get(self.ee as *const _)) }
    }

    pub fn set_ignore_events(&self, on: bool) {
        unsafe { ffi::ecore_evas_ignore_events_set(self.ee, ffi::to_eina_bool(on)) };
    }

    pub fn ignores_events(&self) -> bool {
        unsafe { ffi::from_eina_bool(ffi::ecore_evas_ignore_events_get(self.ee as *const _)) }
    }

    pub fn set_maximized(&self, on: bool) {
        unsafe { ffi::ecore_evas_maximized_set(self.ee, ffi::to_eina_bool(on)) };
    }

    pub fn is_maximized(&self) -> bool {
        unsafe { ffi::from_eina_bool(ffi::ecore_evas_maximized_get(self.ee as *const _)) }
    }

    pub fn set_title(&self, title: &str) {
        unsafe { title.with_c_str(|title| ffi::ecore_evas_title_set(self.ee, title)) };
    }

    pub fn get_title(&self) -> String {
        unsafe { str::raw::from_c_str(ffi::ecore_evas_title_get(self.ee as *const _)) }
    }

    pub fn get_engine_name(&self) -> String {
        unsafe { str::raw::from_c_str(ffi::ecore_evas_engine_name_get(self.ee as *const _)) }
    }

    pub fn show(&self) {
        unsafe { ffi::ecore_evas_show(self.ee) };
    }

    pub fn hide(&self) {
        unsafe { ffi::ecore_evas_hide(self.ee) };
    }

    pub fn activate(&self) {
        unsafe { ffi::ecore_evas_activate(self.ee) };
    }

    pub fn get_position(&self) -> (i32, i32) {
        let (mut x, mut y) = (0, 0);
        unsafe { ffi::ecore_evas_geometry_get(self.ee as *const _, &mut x, &mut y, ptr::mut_null(), ptr::mut_null()) }
        (x as i32, y as i32)
    }

    pub fn get_size(&self) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        unsafe { ffi::ecore_evas_geometry_get(self.ee as *const _, ptr::mut_null(), ptr::mut_null(), &mut w, &mut h) }
        (w as i32, h as i32)
    }

    pub fn get_size_min(&self) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        unsafe { ffi::ecore_evas_size_min_get(self.ee as *const _, &mut w, &mut h) }
        (w as i32, h as i32)
    }

    pub fn get_size_max(&self) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        unsafe { ffi::ecore_evas_size_max_get(self.ee as *const _, &mut w, &mut h) }
        (w as i32, h as i32)
    }

    pub fn get_size_base(&self) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        unsafe { ffi::ecore_evas_size_base_get(self.ee as *const _, &mut w, &mut h) }
        (w as i32, h as i32)
    }

    pub fn get_size_step(&self) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        unsafe { ffi::ecore_evas_size_step_get(self.ee as *const _, &mut w, &mut h) }
        (w as i32, h as i32)
    }

    pub fn set_size_min(&self, w: i32, h: i32) {
        unsafe { ffi::ecore_evas_size_min_set(self.ee, w as libc::c_int, h as libc::c_int) };
    }

    pub fn set_size_max(&self, w: i32, h: i32) {
        unsafe { ffi::ecore_evas_size_max_set(self.ee, w as libc::c_int, h as libc::c_int) };
    }

    pub fn set_size_base(&self, w: i32, h: i32) {
        unsafe { ffi::ecore_evas_size_base_set(self.ee, w as libc::c_int, h as libc::c_int) };
    }

    pub fn set_size_step(&self, w: i32, h: i32) {
        unsafe { ffi::ecore_evas_size_step_set(self.ee, w as libc::c_int, h as libc::c_int) };
    }

    pub fn set_manual_render(&self, on: bool) {
        unsafe { ffi::ecore_evas_manual_render_set(self.ee, ffi::to_eina_bool(on)) };
    }

    pub fn is_manual_render(&self) -> bool {
        unsafe { ffi::from_eina_bool(ffi::ecore_evas_manual_render_get(self.ee as *const _)) }
    }

    pub fn manual_render(&self) {
        unsafe { ffi::ecore_evas_manual_render(self.ee) };
    }

    pub fn input_event_register(&self) {
        unsafe { ffi::ecore_evas_input_event_register(self.ee) };
    }

    pub fn input_event_unregister(&self) {
        unsafe { ffi::ecore_evas_input_event_unregister(self.ee) };
    }

    pub fn get_screen_position(&self) -> (i32, i32) {
        let (mut x, mut y) = (0, 0);
        unsafe { ffi::ecore_evas_screen_geometry_get(self.ee as *const _, &mut x, &mut y, ptr::mut_null(), ptr::mut_null()) }
        (x as i32, y as i32)
    }

    pub fn get_screen_size(&self) -> (i32, i32) {
        let (mut w, mut h) = (0, 0);
        unsafe { ffi::ecore_evas_screen_geometry_get(self.ee as *const _, ptr::mut_null(), ptr::mut_null(), &mut w, &mut h) }
        (w as i32, h as i32)
    }

    pub fn get_screen_dpi(&self) -> (i32, i32) {
        let (mut xdpi, mut ydpi) = (0, 0);
        unsafe { ffi::ecore_evas_screen_dpi_get(self.ee as *const _, &mut xdpi, &mut ydpi) }
        (xdpi as i32, ydpi as i32)
    }

    pub fn get_pointer_position(&self) -> (i32, i32) {
        let (mut x, mut y) = (0, 0);
        unsafe { ffi::ecore_evas_pointer_xy_get(self.ee as *const _, &mut x, &mut y) }
        (x as i32, y as i32)
    }

    pub fn warp_pointer(&self, x: i32, y: i32) {
        unsafe { ffi::ecore_evas_pointer_warp(self.ee as *const _, x as libc::c_int, y as libc::c_int) };
    }
}

#[unsafe_destructor]
impl<'a> Drop for Window<'a> {
    fn drop(&mut self) {
        unsafe {
            ffi::ecore_evas_free(self.ee);
        }
    }
}

macro_rules! event_callbacks {
    ($(($field:ident,
        $extern_set_callback:path,
        $extern_callback: ident,
        $set_callback:ident,
        $unset_callback:ident)),+
    ) => {
        pub trait EventCallback {
            fn call(&self, &Window);
        }

        struct EventCallbacks {
            $($field: Option<Box<EventCallback>>,)+
        }

        impl EventCallbacks {
            fn new() -> EventCallbacks {
                EventCallbacks {
                    $($field: None,)+
                }
            }
        }

        $(extern "C" fn $extern_callback(ee: *mut ffi::Ecore_Evas) {
            println!(stringify!($extern_callback));
            unsafe {
                let window = Window::data_ptr_key().with_c_str(|key| {
                    ffi::ecore_evas_data_get(ee as *const _, key)
                }) as *const Window;
                assert!(!window.is_null());
                match (*window).event_callbacks.$field {
                    Some(ref callback) => {
                        println!("{:p}", callback);
                        callback.call(&*window) // segfault! >_<
                    },
                    None => {
                        $extern_set_callback((*window).ee, None);
                    }
                }
            }
        })+

        impl<'a> Window<'a> {
            $(pub fn $set_callback(&mut self, callback: Box<EventCallback>) -> Option<Box<EventCallback>> {
                println!(stringify!($set_callback));
                unsafe { $extern_set_callback(self.ee, Some($extern_callback)) };
                mem::replace(&mut self.event_callbacks.$field, Some(callback))
            }

            pub fn $unset_callback(&mut self) -> Option<Box<EventCallback>> {
                println!(stringify!($unset_callback));
                unsafe { $extern_set_callback(self.ee, None) };
                self.event_callbacks.$field.take()
            })+
        }
    };
}

event_callbacks! {
//  vtable field    ffi callback setter                          extern "C" callback      callback setter              callback unsetter
    (resize,         ffi::ecore_evas_callback_resize_set,         resize_callback,         set_resize_callback,         unset_resize_callback),
    (move,           ffi::ecore_evas_callback_move_set,           move_callback,           set_move_callback,           unset_move_callback),
    (show,           ffi::ecore_evas_callback_show_set,           show_callback,           set_show_callback,           unset_show_callback),
    (hide,           ffi::ecore_evas_callback_hide_set,           hide_callback,           set_hide_callback,           unset_hide_callback),
    (delete_request, ffi::ecore_evas_callback_delete_request_set, delete_request_callback, set_delete_request_callback, unset_delete_request_callback),
    (destroy,        ffi::ecore_evas_callback_destroy_set,        destroy_callback,        set_destroy_callback,        unset_destroy_callback),
    (focus_in,       ffi::ecore_evas_callback_focus_in_set,       focus_in_callback,       set_focus_in_callback,       unset_focus_in_callback),
    (focus_out,      ffi::ecore_evas_callback_focus_out_set,      focus_out_callback,      set_focus_out_callback,      unset_focus_out_callback),
    (sticky,         ffi::ecore_evas_callback_sticky_set,         sticky_callback,         set_sticky_callback,         unset_sticky_callback),
    (unsticky,       ffi::ecore_evas_callback_unsticky_set,       unsticky_callback,       set_unsticky_callback,       unset_unsticky_callback),
//  (mouse_in,       ffi::ecore_evas_callback_mouse_in_set,       mouse_in_callback,       set_mouse_in_callback,       unset_mouse_in_callback),
//  (mouse_out,      ffi::ecore_evas_callback_mouse_out_set,      mouse_out_callback,      set_mouse_out_callback,      unset_mouse_out_callback),
    (pre_render,     ffi::ecore_evas_callback_pre_render_set,     pre_render_callback,     set_pre_render_callback,     unset_pre_render_callback),
    (post_render,    ffi::ecore_evas_callback_post_render_set,    post_render_callback,    set_post_render_callback,    unset_post_render_callback),
    (pre_free,       ffi::ecore_evas_callback_pre_free_set,       pre_free_callback,       set_pre_free_callback,       unset_pre_free_callback),
    (state_change,   ffi::ecore_evas_callback_state_change_set,   state_change_callback,   set_state_change_callback,   unset_state_change_callback)
}

/// Generates a set of event callbacks
macro_rules! input_callbacks {
    ($(($field:ident,
        $Evas_Event_Info:ty,
        $EventInfo:ident,
        $EVAS_CALLBACK:expr,
        $extern_callback: ident,
        $InputCallback:ident,
        $set_callback:ident,
        $unset_callback:ident)),+
    ) => {
        $(pub trait $InputCallback {
            fn call(&self, &Window, &$EventInfo);
        })+

        /// A vtable of event callback functions
        struct InputCallbacks {
            $($field: Option<Box<$InputCallback>>),+
        }

        impl InputCallbacks {
            /// An empty window event vtable
            fn new() -> InputCallbacks {
                InputCallbacks { $($field: None),+ }
            }
        }

        $(extern "C" fn $extern_callback(
            data: *mut libc::c_void,
            _e: *mut ffi::Evas,
            _obj: *mut ffi::Evas_Object,
            event_info: *mut libc::c_void,
        ) {
            println!(stringify!($extern_callback));
            unsafe {
                let window: &Window = mem::transmute(data);
                match window.input_callbacks.$field {
                    Some(ref callback) => {
                        callback.call(window, &$EventInfo {
                            ptr: event_info as *const _
                        });
                    },
                    None => {
                        ffi::evas_object_event_callback_del(
                            window.object, $EVAS_CALLBACK, Some($extern_callback),
                        );
                    },
                }
            }
        })+

        impl<'a> Window<'a> {
            $(pub fn $set_callback(&mut self, callback: Box<$InputCallback>) -> Option<Box<$InputCallback>> {
                unsafe {
                    ffi::evas_object_event_callback_add(
                        self.object, $EVAS_CALLBACK, Some($extern_callback), self as *mut _ as *const _,
                    );
                }
                mem::replace(&mut self.input_callbacks.$field, Some(callback))
            }

            pub fn $unset_callback(&mut self) -> Option<Box<$InputCallback>> {
                unsafe {
                    ffi::evas_object_event_callback_del(
                        self.object, $EVAS_CALLBACK, Some($extern_callback),
                    );
                }
                self.input_callbacks.$field.take()
            })+
        }
    }
}

input_callbacks! {
//  vtable field    ffi event info struct         event info    callback ffi specifier           extern "C" callback   callback trait      callback setter             callback unsetter
    (mouse_down,    ffi::Evas_Event_Mouse_Down,   MouseDown,    ffi::EVAS_CALLBACK_MOUSE_DOWN,   mouse_down_callback,  MouseDownCallback,  set_mouse_down_callback,    unset_mouse_down_callback),
    (mouse_up,      ffi::Evas_Event_Mouse_Up,     MouseUp,      ffi::EVAS_CALLBACK_MOUSE_UP,     mouse_up_callback,    MouseUpCallback,    set_mouse_up_callback,      unset_mouse_up_callback),
    (mouse_in,      ffi::Evas_Event_Mouse_In,     MouseIn,      ffi::EVAS_CALLBACK_MOUSE_IN,     mouse_in_callback,    MouseInCallback,    set_mouse_in_callback,      unset_mouse_in_callback),
    (mouse_out,     ffi::Evas_Event_Mouse_Out,    MouseOut,     ffi::EVAS_CALLBACK_MOUSE_OUT,    mouse_out_callback,   MouseOutCallback,   set_mouse_out_callback,     unset_mouse_out_callback),
    (mouse_move,    ffi::Evas_Event_Mouse_Move,   MouseMove,    ffi::EVAS_CALLBACK_MOUSE_MOVE,   mouse_move_callback,  MouseMoveCallback,  set_mouse_move_callback,    unset_mouse_move_callback),
    (mouse_wheel,   ffi::Evas_Event_Mouse_Wheel,  MouseWheel,   ffi::EVAS_CALLBACK_MOUSE_WHEEL,  mouse_wheel_callback, MouseWheelCallback, set_mouse_wheel_callback,   unset_mouse_wheel_callback),
    (multi_down,    ffi::Evas_Event_Multi_Down,   MultiDown,    ffi::EVAS_CALLBACK_MULTI_DOWN,   multi_down_callback,  MultiDownCallback,  set_multi_down_callback,    unset_multi_down_callback),
    (multi_up,      ffi::Evas_Event_Multi_Up,     MultiUp,      ffi::EVAS_CALLBACK_MULTI_UP,     multi_up_callback,    MultiUpCallback,    set_multi_up_callback,      unset_multi_up_callback),
    (multi_move,    ffi::Evas_Event_Multi_Move,   MultiMove,    ffi::EVAS_CALLBACK_MULTI_MOVE,   multi_move_callback,  MultiMoveCallback,  set_multi_move_callback,    unset_multi_move_callback),
    (key_down,      ffi::Evas_Event_Key_Down,     KeyDown,      ffi::EVAS_CALLBACK_KEY_DOWN,     key_down_callback,    KeyDownCallback,    set_key_down_callback,      unset_key_down_callback),
    (key_up,        ffi::Evas_Event_Key_Up,       KeyUp,        ffi::EVAS_CALLBACK_KEY_UP,       key_up_callback,      KeyUpCallback,      set_key_up_callback,        unset_key_up_callback),
//  (render_post,   ffi::Evas_Event_Render_Post,  RenderPost,   ffi::EVAS_CALLBACK_RENDER_POST,  render_post_callback, RenderPostCallback, set_render_post_callback,   unset_render_post_callback),
    (hold,          ffi::Evas_Event_Hold,         Hold,         ffi::EVAS_CALLBACK_HOLD,         hold_callback,        HoldCallback,       set_hold_callback,          unset_hold_callback)
}

pub type MouseButton = libc::c_int;
pub type TimeStamp = libc::c_uint;
pub type Coord = ffi::Evas_Coord;

pub struct Point {
    pub x: libc::c_int,
    pub y: libc::c_int,
}

impl Point {
    fn from_evas(point: ffi::Evas_Point) -> Point {
        match point {
            ffi::Evas_Point { x, y } => Point { x: x, y: y },
        }
    }
}

pub struct CoordPoint {
    pub x: Coord,
    pub y: Coord,
}

impl CoordPoint {
    fn from_evas(point: ffi::Evas_Coord_Point) -> CoordPoint {
        match point {
            ffi::Evas_Coord_Point { x, y } => CoordPoint { x: x, y: y },
        }
    }
}

pub struct CoordPrecisionPoint {
    pub x: Coord,
    pub y: Coord,
    pub xsub: libc::c_double,
    pub ysub: libc::c_double,
}

impl CoordPrecisionPoint {
    fn from_evas(point: ffi::Evas_Coord_Precision_Point) -> CoordPrecisionPoint {
        match point {
            ffi::Evas_Coord_Precision_Point { x, y, xsub, ysub } => {
                CoordPrecisionPoint { x: x, y: y, xsub: xsub, ysub: ysub }
            },
        }
    }
}

pub struct Position {
    pub output: Point,
    pub canvas: CoordPoint,
}

impl Position {
    fn from_evas(position: ffi::Evas_Position) -> Position {
        match position {
            ffi::Evas_Position { output, canvas } => {
                Position {
                    output: Point::from_evas(output),
                    canvas: CoordPoint::from_evas(canvas),
                }
            },
        }
    }
}

pub struct PrecisionPosition {
    pub output: Point,
    pub canvas: CoordPrecisionPoint,
}

impl PrecisionPosition {
    fn from_evas(position: ffi::Evas_Precision_Position) -> PrecisionPosition {
        match position {
            ffi::Evas_Precision_Position { output, canvas } => {
                PrecisionPosition {
                    output: Point::from_evas(output),
                    canvas: CoordPrecisionPoint::from_evas(canvas),
                }
            },
        }
    }
}

bitflags! {
    flags EventFlags: libc::c_uint {
        static EventFlagNone = ffi::EVAS_EVENT_FLAG_NONE,
        static EventFlagOnHold = ffi::EVAS_EVENT_FLAG_ON_HOLD,
        static EventFlagOnScroll = ffi::EVAS_EVENT_FLAG_ON_SCROLL
    }
}

bitflags! {
    flags ButtonFlags: libc::c_uint {
        static ButtonNone = ffi::EVAS_BUTTON_NONE,
        static ButtonDoubleClick = ffi::EVAS_BUTTON_DOUBLE_CLICK,
        static ButtonTripleClick = ffi::EVAS_BUTTON_TRIPLE_CLICK
    }
}

pub struct Modifier {
    ptr: *const ffi::Evas_Modifier,
}

impl Modifier {
    pub fn is_set(&self, keyname: &str) -> bool {
        ffi::from_eina_bool(unsafe {
            keyname.with_c_str(|name| {
                ffi::evas_key_modifier_is_set(self.ptr, name)
            })
        })
    }
}

pub struct Lock {
    ptr: *const ffi::Evas_Lock,
}

impl Lock {
    pub fn is_set(&self, keyname: &str) -> bool {
        ffi::from_eina_bool(unsafe {
            keyname.with_c_str(|name| {
                ffi::evas_key_lock_is_set(self.ptr, name)
            })
        })
    }
}

/// Generates a safe wrapper around an Evas event info struct
macro_rules! event_info_wrapper {
    (struct $EventInfo:ident($Evas_Event_Info:ty) {
        $($field:ident: $Field:ty = $body:expr),+
    }) => {
        pub struct $EventInfo {
            ptr: *const $Evas_Event_Info,
        }

        impl $EventInfo {
            $(pub fn $field(&self) -> $Field {
                let $field = unsafe { (*self.ptr).$field };
                $body
            })+
        }
    }
}

event_info_wrapper! {
    struct MouseDown(ffi::Evas_Event_Mouse_Down) {
        button:         MouseButton = button,
        output:         Point = Point::from_evas(output),
        canvas:         CoordPoint = CoordPoint::from_evas(canvas),
        // data:        *mut libc::c_void = _,
        modifiers:      Modifier = Modifier { ptr: modifiers as *const _ },
        locks:          Lock = Lock { ptr: locks as *const _ },
        flags:          ButtonFlags = ButtonFlags::from_bits(flags).unwrap(),
        timestamp:      TimeStamp = timestamp,
        event_flags:    EventFlags = EventFlags::from_bits(event_flags).unwrap()
        // dev:         *mut Evas_Device = _,
        // event_src:   *mut Evas_Object = _,
    }
}

event_info_wrapper! {
    struct MouseUp(ffi::Evas_Event_Mouse_Up) {
        button:         MouseButton = button,
        output:         Point = Point::from_evas(output),
        canvas:         CoordPoint = CoordPoint::from_evas(canvas),
        // data:        *mut libc::c_void = _,
        modifiers:      Modifier = Modifier { ptr: modifiers as *const _ },
        locks:          Lock = Lock { ptr: locks as *const _ },
        flags:          ButtonFlags = ButtonFlags::from_bits(flags).unwrap(),
        timestamp:      TimeStamp = timestamp,
        event_flags:    EventFlags = EventFlags::from_bits(event_flags).unwrap()
        // dev:         *mut Evas_Device = _,
        // event_src:   *mut Evas_Object = _,
    }
}

event_info_wrapper! {
    struct MouseIn(ffi::Evas_Event_Mouse_In) {
        buttons:        MouseButton = buttons,
        output:         Point = Point::from_evas(output),
        canvas:         CoordPoint = CoordPoint::from_evas(canvas),
        // data:        *mut libc::c_void = _,
        modifiers:      Modifier = Modifier { ptr: modifiers as *const _ },
        locks:          Lock = Lock { ptr: locks as *const _ },
        timestamp:      TimeStamp = timestamp,
        event_flags:    EventFlags = EventFlags::from_bits(event_flags).unwrap()
        // dev:         *mut Evas_Device = _,
        // event_src:   *mut Evas_Object = _,
    }
}

event_info_wrapper! {
    struct MouseOut(ffi::Evas_Event_Mouse_Out) {
        buttons:        MouseButton = buttons,
        output:         Point = Point::from_evas(output),
        canvas:         CoordPoint = CoordPoint::from_evas(canvas),
        // data:        *mut libc::c_void = _,
        modifiers:      Modifier = Modifier { ptr: modifiers as *const _ },
        locks:          Lock = Lock { ptr: locks as *const _ },
        timestamp:      TimeStamp = timestamp,
        event_flags:    EventFlags = EventFlags::from_bits(event_flags).unwrap()
        // dev:         *mut Evas_Device = _,
        // event_src:   *mut Evas_Object = _,
    }
}

event_info_wrapper! {
    struct MouseMove(ffi::Evas_Event_Mouse_Move) {
        buttons:        MouseButton = buttons,
        cur:            Position = Position::from_evas(cur),
        prev:           Position = Position::from_evas(prev),
        // data:        *mut libc::c_void = _,
        modifiers:      Modifier = Modifier { ptr: modifiers as *const _ },
        locks:          Lock = Lock { ptr: locks as *const _ },
        timestamp:      TimeStamp = timestamp,
        event_flags:    EventFlags = EventFlags::from_bits(event_flags).unwrap()
        // dev:         *mut Evas_Device = _,
        // event_src:   *mut Evas_Object = _,
    }
}

event_info_wrapper! {
    struct MouseWheel(ffi::Evas_Event_Mouse_Wheel) {
        direction:      libc::c_int = direction,
        z:              libc::c_int = z,
        output:         Point = Point::from_evas(output),
        canvas:         CoordPoint = CoordPoint::from_evas(canvas),
        // data:        *mut libc::c_void = _,
        modifiers:      Modifier = Modifier { ptr: modifiers as *const _ },
        locks:          Lock = Lock { ptr: locks as *const _ },
        timestamp:      TimeStamp = timestamp,
        event_flags:    EventFlags = EventFlags::from_bits(event_flags).unwrap()
        // dev:         *mut Evas_Device = _,
        // event_src:   *mut Evas_Object = _,
    }
}

event_info_wrapper! {
    struct MultiDown(ffi::Evas_Event_Multi_Down) {
        device:         libc::c_int = device,
        radius:         libc::c_double = radius,
        radius_x:       libc::c_double = radius_x,
        radius_y:       libc::c_double = radius_y,
        pressure:       libc::c_double = pressure,
        angle:          libc::c_double = angle,
        output:         Point = Point::from_evas(output),
        canvas:         CoordPrecisionPoint = CoordPrecisionPoint::from_evas(canvas),
        // data:        *mut libc::c_void = _,
        modifiers:      Modifier = Modifier { ptr: modifiers as *const _ },
        locks:          Lock = Lock { ptr: locks as *const _ },
        flags:          ButtonFlags = ButtonFlags::from_bits(flags).unwrap(),
        timestamp:      TimeStamp = timestamp,
        event_flags:    EventFlags = EventFlags::from_bits(event_flags).unwrap()
        // dev:         *mut Evas_Device = _,
    }
}

event_info_wrapper! {
    struct MultiUp(ffi::Evas_Event_Multi_Up) {
        device:         libc::c_int = device,
        radius:         libc::c_double = radius,
        radius_x:       libc::c_double = radius_x,
        radius_y:       libc::c_double = radius_y,
        pressure:       libc::c_double = pressure,
        angle:          libc::c_double = angle,
        output:         Point = Point::from_evas(output),
        canvas:         CoordPrecisionPoint = CoordPrecisionPoint::from_evas(canvas),
        // data:        *mut libc::c_void = _,
        modifiers:      Modifier = Modifier { ptr: modifiers as *const _ },
        locks:          Lock = Lock { ptr: locks as *const _ },
        flags:          ButtonFlags = ButtonFlags::from_bits(flags).unwrap(),
        timestamp:      TimeStamp = timestamp,
        event_flags:    EventFlags = EventFlags::from_bits(event_flags).unwrap()
        // dev:         *mut Evas_Device = _,
    }
}

event_info_wrapper! {
    struct MultiMove(ffi::Evas_Event_Multi_Move) {
        device:         libc::c_int = device,
        radius:         libc::c_double = radius,
        radius_x:       libc::c_double = radius_x,
        radius_y:       libc::c_double = radius_y,
        pressure:       libc::c_double = pressure,
        angle:          libc::c_double = angle,
        cur:            PrecisionPosition = PrecisionPosition::from_evas(cur),
        // data:        *mut libc::c_void = _,
        modifiers:      Modifier = Modifier { ptr: modifiers as *const _ },
        locks:          Lock = Lock { ptr: locks as *const _ },
        timestamp:      TimeStamp = timestamp,
        event_flags:    EventFlags = EventFlags::from_bits(event_flags).unwrap()
        // dev:         *mut Evas_Device = _,
    }
}

event_info_wrapper! {
    struct KeyDown(ffi::Evas_Event_Key_Down) {
        keyname:        String = unsafe { str::raw::from_c_str(keyname as *const _) },
        // data:        *mut libc::c_void = _,
        modifiers:      Modifier = Modifier { ptr: modifiers as *const _ },
        locks:          Lock = Lock { ptr: locks as *const _ },
        key:            String = unsafe { str::raw::from_c_str(key) },
        string:         String = unsafe { str::raw::from_c_str(string) },
        compose:        String = unsafe { str::raw::from_c_str(compose) },
        timestamp:      TimeStamp = timestamp,
        event_flags:    EventFlags = EventFlags::from_bits(event_flags).unwrap(),
        // dev:         *mut Evas_Device = _,
        keycode:        libc::c_uint = keycode
    }
}

event_info_wrapper! {
    struct KeyUp(ffi::Evas_Event_Key_Up) {
        keyname:        String = unsafe { str::raw::from_c_str(keyname as *const _) },
        // data:        *mut libc::c_void = _,
        modifiers:      Modifier = Modifier { ptr: modifiers as *const _ },
        locks:          Lock = Lock { ptr: locks as *const _ },
        key:            String = unsafe { str::raw::from_c_str(key) },
        string:         String = unsafe { str::raw::from_c_str(string) },
        compose:        String = unsafe { str::raw::from_c_str(compose) },
        timestamp:      TimeStamp = timestamp,
        event_flags:    EventFlags = EventFlags::from_bits(event_flags).unwrap(),
        // dev:         *mut Evas_Device = _,
        keycode:        libc::c_uint = keycode
    }
}

// event_info_wrapper! {
//     struct RenderPost(ffi::Evas_Event_Render_Post) {
//         updated_area: *mut Eina_List
//     }
// }

event_info_wrapper! {
    struct Hold(ffi::Evas_Event_Hold) {
        hold:           libc::c_int = hold,
        // data:        *mut libc::c_void = _,
        timestamp:      TimeStamp = timestamp,
        event_flags:    EventFlags = EventFlags::from_bits(event_flags).unwrap()
        // dev:         *mut Evas_Device = _,
        // event_src:   *mut Evas_Object = _,
    }
}
