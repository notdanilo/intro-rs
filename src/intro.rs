use super::math_util;
use super::gl;
use super::gl_util;
use super::random;
use core::arch::x86;

use gl::CVoid;
use core::mem::{size_of,transmute};
use core::ops::{Add,Sub,Mul};

pub const FP_0_01 :f32 = 0.0100097656f32;  //    0.01
pub const FP_0_02 :f32 = 0.0200195313f32;     // 0.02f    0x3ca40000
pub const FP_0_05 :f32 = 0.0500488281f32;  //
pub const FP_0_20 : f32 = 0.2001953125f32;

pub const FP_1_32  : f32 = 1.3203125000f32;     // 1.32f    0x3fa90000
pub const FP_1_54 : f32 = 1.5390625000f32;

pub const CAMERA_POS_IDX : usize = 80*2;
pub const CAMERA_ROT_IDX : usize = 80*2+1;
pub const CAMERA_CUT_INFO : usize = 80*2+2;

pub const num_spheres : usize = 80;
pub const sphere_extras : usize = 2;

static mut shader_prog : gl::GLuint = 0;
static mut vertex_array_id : gl::GLuint = 0;

static mut rng : random::Rng = random::Rng{seed: core::num::Wrapping(21431249)};

static mut global_spheres: [ [ f32; 4]; (num_spheres+sphere_extras)*2] = [ [ 0f32; 4]; (num_spheres+sphere_extras)*2 ];  

fn smooth( pixels: &mut [ f32; 512*513*4 ]) {
    unsafe{
        let mut xy = 0;
        loop{
            let offset = xy*4;
            let mut val  = *pixels.get_unchecked( offset );
            val += pixels.get_unchecked( offset+4 );
            val += pixels.get_unchecked( offset+2048 );
            val += pixels.get_unchecked( offset+2052 );
            *pixels.get_unchecked_mut( offset ) = val / 4.0;
            xy += 1;
            if xy == 511*511 { break; }
        }
    }
}

//static mut gpixels : [ u8; 512*512*4 ] = [ 0; 512*512*4 ];      // large ones are OK as long as they are 0. Otherwise crinkler chokes
static mut src_terrain  : [ f32; 512*513*4 ] = [ 0.0; 512*513*4 ];
static mut tex_buffer_id : gl::GLuint = 0;

#[cfg(feature = "logger")]
static mut glbl_shader_code : [ u8;25000] = [0; 25000];

static mut old_x : i32 = 0;
static mut old_y : i32 = 0;
static mut moving_camera : bool  = false;
static mut rotating_camera : bool  = false;

static mut camera_velocity : [ f32; 4] = [ 0.0; 4];
static mut camera_rot_speed : [ f32; 4] = [ 0.0; 4];

static mut pivot_cam_centre : [ f32; 3] = [ 0.0; 3];
static mut pivot_cam_dist : [ f32; 3] = [ 0.0; 3];
static mut pivot_cam_angle : [ f32; 3] = [ 0.0; 3];

static mut camera_mode : u32 = 0;

#[cfg(feature = "logger")]
pub fn set_pos( x: i32, y: i32, ctrl : bool ) {
    unsafe{
        if moving_camera {
            if ctrl{
                global_spheres[ CAMERA_POS_IDX ][ 1 ] += ( y-old_y) as f32 / 32.0;
            } else {
                global_spheres[ CAMERA_POS_IDX ][ 0 ] += ( x-old_x) as f32 / 32.0;
                global_spheres[ CAMERA_POS_IDX ][ 2 ] += ( y-old_y) as f32 / 32.0;
            }
        } else if rotating_camera {
            global_spheres[ CAMERA_ROT_IDX ][ 0 ] += ( y-old_y) as f32 / 1024.0;
            global_spheres[ CAMERA_ROT_IDX ][ 1 ] += ( x-old_x) as f32 / 1024.0;
//            world_rot[ 2 ] += ( y-old_y) as f32 / 32.0;
    
        }
        old_x = x;
        old_y = y;
    }
}

#[cfg(feature = "logger")]
pub fn rbutton_down( x: i32, y: i32 ) {
    unsafe{ 
        old_x = x;
        old_y = y;
        moving_camera = false;
        rotating_camera = true;
    }
}

#[cfg(feature = "logger")]
pub fn rbutton_up( ) {
    setup_random_camera();
    unsafe{ 
        rotating_camera = false;
    }
}

#[cfg(feature = "logger")]
pub fn lbutton_down( x: i32, y: i32 ) {
    unsafe{ 
        old_x = x;
        old_y = y;
        moving_camera = true;
        rotating_camera = false;
    }
}

#[cfg(feature = "logger")]
pub fn lbutton_up( ) {
    unsafe{ 
        moving_camera = false;
    }
    unsafe{ super::log!( "Camera: ", global_spheres[ CAMERA_POS_IDX ][ 0 ], global_spheres[ CAMERA_POS_IDX ][ 1 ], global_spheres[ CAMERA_POS_IDX ][ 2 ]); }
}

static mut r3_pos : usize = 0;

fn set_r3( dest : &mut[ f32 ; 4 ], crng : &mut random::Rng, a: f32, b: f32, c: f32, offset: f32 ) {
    // tried turning into a loop -> crinkled version grew 60bytes!
    let x = crng.next_f32();
    let z = crng.next_f32();
    dest[ 0 ] = (x-offset)*a;
    dest[ 1 ] = (crng.next_f32()-offset)*b;
    dest[ 2 ] = (z-offset)*c;
    unsafe{
        // we only ever calculate the position scaled by 512 ( by the unoffset values )
        r3_pos = (((z*512f32) as usize *512)+(x*512f32) as usize)*4;
    }
}

pub fn prepare() -> () {
    let mut error_message : [i8;100] = [ 0; 100];
     let vtx_shader_src : &'static str = "#version 330 core
    layout (location = 0) in vec3 Pos;
    void main()
    {
     gl_Position = vec4(Pos, 1.0);
    }\0";

    let spheres : &mut[ [ f32; 4]; (num_spheres+sphere_extras)*2];  
    unsafe{
        spheres  = &mut global_spheres;
    }
    
    let vtx_shader : u32;
    let frag_shader : u32;
    unsafe{ super::log!( "Load shader !"); };

    #[cfg(not(feature = "logger"))]
    {
        vtx_shader = gl_util::shader_from_source( vtx_shader_src.as_ptr(), gl::VERTEX_SHADER, &mut error_message ).unwrap();
        frag_shader  = gl_util::shader_from_source( super::shaders::frag_shader_src.as_ptr(), gl::FRAGMENT_SHADER,  &mut error_message ).unwrap();
        unsafe{
            shader_prog = gl_util::program_from_shaders(vtx_shader, frag_shader, &mut error_message ).unwrap();
        }
    }

    #[cfg(feature = "logger")]
    {
        vtx_shader = match gl_util::shader_from_source( vtx_shader_src.as_ptr(), gl::VERTEX_SHADER, &mut error_message ) {
            Some( shader ) => shader,
            None => { super::show_error( error_message.as_ptr()  ); 0 }
        };
        unsafe{  
            super::util::read_file( "shader.glsl\0", &mut glbl_shader_code); 
            frag_shader  = match gl_util::shader_from_source( glbl_shader_code.as_ptr(), gl::FRAGMENT_SHADER,  &mut error_message ) {
                Some( shader ) => shader,
                None => { super::show_error( error_message.as_ptr() ); 0 }
            };
        }
        unsafe{
            shader_prog = match gl_util::program_from_shaders(vtx_shader, frag_shader, &mut error_message ) {
                Some( prog ) => prog,
                None => { super::show_error( error_message.as_ptr() ); 0 }
            };
        }
    }

    unsafe{
        super::log!( "Build terrain!");
        // COULD restructure this to create the points for each iteration instead of storing into array. Very slow but could save some bytes 
        let mut rng_terrain : random::Rng = random::Rng{seed: core::num::Wrapping(9231249)};
        let mut lumps : [[f32;4];50] = [[0f32;4];50];
        let num_lumps = 50;

        let mut nl = 0;
        loop{
            set_r3( lumps.get_unchecked_mut(nl), &mut rng_terrain,1f32,1f32,1f32, 0.0 );
            nl += 1;
            if nl == num_lumps {break}
        }

        let  mut i = 0;
        loop{
            set_r3( spheres.get_unchecked_mut(0), &mut rng_terrain,1f32,1f32,1f32, 0.0 );
            let x = spheres.get_unchecked_mut(0)[0];
            let z = spheres.get_unchecked_mut(0)[2];

            let mut charge = 0.0;
            nl = 0;
            loop{
                let lmp = lumps.get(nl).unwrap();
                let dist = (x-lmp[0])*(x-lmp[0]) + (z-lmp[2])*(z-lmp[2]);
                charge += lmp[1]*0.0001/dist;
                nl += 1;
                if nl == num_lumps { break;}
            }
            *src_terrain.get_unchecked_mut( r3_pos ) += charge;
            if *src_terrain.get_unchecked( r3_pos ) > 1.0  {
                *src_terrain.get_unchecked_mut( r3_pos ) = 1.0
            }
            i += 1;
            if i== 1_000_000 { break}
        }

        smooth( &mut src_terrain); 
        let x : u32 = 256 + 130;
        let y : u32 = 256 + 191;
        let pos = ((y*512)+(x))*4+1;
        super::log!( "!!!!");
        super::log!( "", src_terrain[ (pos-1) as usize] );
        super::log!( "!!!!");
        *src_terrain.get_unchecked_mut( pos as usize ) = 1.0f32;

        let mut idx = 0;
        loop {
            loop{
                set_r3( spheres.get_unchecked_mut(idx*2), &mut rng_terrain,512f32,512f32,512f32, 0.0 );
                if *src_terrain.get_unchecked( r3_pos ) > 0.3f32 {
                    spheres.get_unchecked_mut(idx*2)[ 1 ] = *src_terrain.get_unchecked( r3_pos )*60.0-12.1;
                    spheres.get_unchecked_mut(idx*2)[ 3 ] = 18.0f32;
                    spheres.get_unchecked_mut(idx*2+1)[ 0 ] = FP_0_02;
                    spheres.get_unchecked_mut(idx*2+1)[ 1 ] = FP_0_02;
                    spheres.get_unchecked_mut(idx*2+1)[ 2 ] = FP_0_02;
                    spheres.get_unchecked_mut(idx*2+1)[ 3 ] = FP_1_32;
                    break;
                }
            }
            idx += 1;
            if idx == num_spheres { break;}
        }
    }



    let mut vertex_buffer_id : gl::GLuint = 0;
    unsafe{
        // Create the map texture
        gl::GenTextures( 1, &mut tex_buffer_id );
        gl::ActiveTexture(gl::TEXTURE0);
        gl::BindTexture( gl::TEXTURE_2D, tex_buffer_id );
        gl::TexImage2D(gl::TEXTURE_2D, 0, gl::RGB, 512, 512, 0, gl::RGBA, gl::FLOAT, src_terrain.as_ptr() as *const CVoid);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MAG_FILTER, gl::NEAREST as i32);
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_MIN_FILTER, gl::NEAREST as i32);

        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_S, gl::CLAMP_TO_EDGE as i32 );
        gl::TexParameteri(gl::TEXTURE_2D, gl::TEXTURE_WRAP_T, gl::CLAMP_TO_EDGE as i32 );
    }
}


static mut delay_counter : u32 = 0;
static mut play_pos : usize = 0;

fn update_world() {
    
    unsafe{
        if play_pos >= 21 && play_pos <= 33 {
            camera_mode = 1;
        } else {
            camera_mode = 0;
        }
        delay_counter = *sequence.get_unchecked( play_pos ) as u32*60;
        let seed : u32 = (*sequence.get_unchecked( play_pos+38 )).into();
        super::log!( "Camera", seed as f32, camera_mode as f32);
        setup_camera( seed, camera_mode as u8 );
        play_pos += 1;
    }
}

static mut cam_count : u32 = 1100;          // (1753 0 )

fn setup_random_camera( ) {
    let seed : u32;
    unsafe{ 
        cam_count += 1;
        setup_camera( cam_count, 1);
        camera_mode = 1;

    }   
}


fn setup_camera( seed : u32, mode : u8) {
    unsafe{ super::log!( "Setup Camera: ", mode as f32, seed as f32 ); }

    let mut crng : random::Rng = random::Rng{seed: core::num::Wrapping(9231249+seed)};
    unsafe{ super::log!( "Setup Camera: ", 2.0 ); }
    unsafe{
        super::log!( "Setup Camera: ", 11.0 );
        set_r3( &mut global_spheres[ CAMERA_POS_IDX ], &mut crng, 512f32, 512f32, 512f32, 0.0);
        global_spheres[ CAMERA_POS_IDX ][ 1 ] = (*src_terrain.get_unchecked( r3_pos ))*60.0-2.1+crng.next_f32()*5.0;
        super::log!( "Setup Camera: ", 12.0 );
        set_r3( &mut global_spheres[ CAMERA_ROT_IDX ], &mut crng, FP_1_54, 3.15, FP_0_05, 0.5 );
        set_r3( &mut camera_velocity, &mut crng, FP_0_20, FP_0_05, FP_0_20, 0.5);
        set_r3( &mut camera_rot_speed, &mut crng, 0.002, 0.001, 0.001, 0.5 );

        if mode == 1 {
            let scale = crng.next_f32()*10f32;
            global_spheres[ CAMERA_ROT_IDX ][ 0 ]  =  (crng.next_f32()-0.5)*FP_1_54;
    
            camera_velocity[ 0 ] *= FP_0_01;
            camera_rot_speed[ 1 ] *= 5.0f32;

            pivot_cam_dist[ 0 ] = (1.1f32-crng.next_f32())*scale;
            pivot_cam_angle[1] = global_spheres[ CAMERA_ROT_IDX ][ 1 ];
            pivot_cam_centre[ 1 ] = 25.8 + crng.next_f32()*0.4f32*scale;
        }
    }
    unsafe{ super::log!( "Setup Camera: ", 3.0 ); }
    
}

//random camera centre 130.5525, 27.7635, 191.6042

// 38*2  ( save 5 bytes in compressed space by grouping by type to get more compressability)
static sequence : &[u16] = &[
// CONFIRMED SEQUENCE
// close shots of water - glimpses of land
2,       //water wobbles
2,        //water wobbles
4,       //water wobbles
2,        //water wobbles
2,        //water wobbles
6,         //need better pan up from water shot   18

// forward shots
3,            //idx 6
3,       
5,         // low forward beach shot
4,       
14,          // long turning shot
3,         // forward tuning shot nice
3,         // nice color foward pan
6,       
6,       // 33

// left
4,         // idx 15
8,       // animate sphere at this point. Could be longer

// up to pan around
3,       // pan color angle forward     // idx 17
8,       // rising upo high from water, nice long shadow
6,       // nice high shot looking down
6,       // nice high look downn into holo map

// far   
4,    // SPIN CAM_START   // idx 21
4, 
3, 

//    nearer
3, 

// very close
3, 
5, 
3, 
3, 
3, 
3, 
10,

// Leave holo
2, // SPIN CAM_LAST   // idx 32

// Final pull back
4,  // nice pull back
4,  // pan back over sphere
2,  // very nice backward pass 
2,  // pan back water  
20,  // big wide back pan color


// CONFIRMED SEQUENCE
// close shots of water - glimpses of land
64,         //water wobbles
434,         //water wobbles
65,         //water wobbles
798,         //water wobbles
436,         //water wobbles
1187,         //need better pan up from water shot   18

// forward shots
317,           //idx 6
298, 
1649,       // low forward beach shot
909, 
 724,         // long turning shot
1453,       // forward tuning shot nice
1007,       // nice color foward pan
723, 
1046,     // 33

// left
123,           // idx 15
1299,        // animate sphere at this point. Could be longer

// up to pan around
1120,       // pan color angle forward     // idx 17
636,        // rising upo high from water, nice long shadow
613,        // nice high shot looking down
1006,       // nice high look downn into holo map

// far   
449,          // SPIN CAM_START   // idx 21
729,
398,

//    nearer
353,

// very close
942,
666,
345,
420,
495,
983,
741,

// Leave holo
1490,     // SPIN CAM_LAST   // idx 32

// Final pull back
166,        // nice pull back
151,        // pan back over sphere
691,        // very nice backward pass 
1112,       // pan back water  
 1261,       // big wide back pan color




];

pub fn frame( now : f32 ) -> () {
    unsafe {
        if delay_counter == 0 {
            update_world( );
            global_spheres[ CAMERA_CUT_INFO ][ 1 ] = 0f32;

        }
        delay_counter -= 1;
        global_spheres[ CAMERA_CUT_INFO ][ 1 ] += 1f32;
    }

    unsafe{
        // let mut dst:x86::__m128 = core::arch::x86::_mm_load_ps(global_spheres[ CAMERA_ROT_IDX ].as_mut_ptr());
        // let mut src:x86::__m128 = core::arch::x86::_mm_load_ps(camera_rot_speed.as_mut_ptr());
        // dst = core::arch::x86::_mm_add_ps( dst, src);
        // core::arch::x86::_mm_store_ss( (&mut global_spheres[ CAMERA_ROT_IDX ]).as_mut_ptr(), dst );
        global_spheres[ CAMERA_ROT_IDX ][ 0 ] += camera_rot_speed[ 0 ];
        global_spheres[ CAMERA_ROT_IDX ][ 1 ] += camera_rot_speed[ 1 ];
        global_spheres[ CAMERA_ROT_IDX ][ 2 ] += camera_rot_speed[ 2 ];
        if camera_mode == 0 {
            // dst = core::arch::x86::_mm_load_ps(global_spheres[ CAMERA_POS_IDX ].as_mut_ptr());
            // src = core::arch::x86::_mm_load_ps(camera_velocity.as_mut_ptr());
            // dst = core::arch::x86::_mm_add_ps( dst, src);
            // core::arch::x86::_mm_store_ss( (&mut global_spheres[ CAMERA_POS_IDX ]).as_mut_ptr(), dst );
            global_spheres[ CAMERA_POS_IDX ][ 0 ] += camera_velocity[ 0 ];
            global_spheres[ CAMERA_POS_IDX ][ 1 ] += camera_velocity[ 1 ];
            global_spheres[ CAMERA_POS_IDX ][ 2 ] += camera_velocity[ 2 ];

        }  else if camera_mode == 1 {
            let angle = global_spheres[ CAMERA_ROT_IDX ][ 1 ] - 3.14f32 / 2.0f32; //pivot_cam_angle[1];
            global_spheres[ CAMERA_POS_IDX ][ 0 ] = 130.5 + 256.0 + math_util::cos(angle )*pivot_cam_dist[ 0 ]*pivot_cam_dist[ 0 ];
            global_spheres[ CAMERA_POS_IDX ][ 1 ] = pivot_cam_centre[ 1 ];
            global_spheres[ CAMERA_POS_IDX ][ 2 ] = 191.5 + 256.0 - math_util::sin(angle)*pivot_cam_dist[ 0 ]*pivot_cam_dist[ 0 ];
            pivot_cam_dist[ 0 ] += camera_velocity[ 0 ]*1.0f32;
        }
        global_spheres[ CAMERA_CUT_INFO ][ 0 ] = delay_counter as f32;
        global_spheres[ CAMERA_CUT_INFO ][ 2 ] = now;
    }

    unsafe{
        gl::UseProgram(shader_prog);
        let shperes_loc : i32 = gl::GetUniformLocation(shader_prog, "sp\0".as_ptr());
        gl::Uniform4fv(shperes_loc, (num_spheres+sphere_extras) as i32 * 2, transmute::<_,*const gl::GLfloat>( global_spheres.as_ptr() ) );
        gl::Recti( -1, -1, 1, 1 );
    }
}