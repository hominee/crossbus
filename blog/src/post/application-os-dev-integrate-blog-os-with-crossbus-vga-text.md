# Integrate Blog OS with CrossBus VGA Text 

The [Blog OS](https://os.phil-opp.com/) is a great series of source to 
create a small operating system in rust language.

And CrossBus is a lower level actor computing model compatible with 
bare metal, can efficiently and asynchronously transmit / share data between
components which may make a difference in OS development.

### Integration Target
Only the first 3 posts (by [Vga-text](https://os.phil-opp.com/vga-text-mode/)) will be touched in the section. 
So if you don't familiar with [post-1](https://os.phil-opp.com/freestanding-rust-binary/) or [post-2](https://os.phil-opp.com/minimal-rust-kernel/) 
you'd better take a look beforehand.

### Pre-Requisites
Since crossbus use rust alloc crate, 

- we must add `alloc` to the `build-std` field of `.cargo/config.toml`:

```toml 
build-std = ["core", "compiler_builtins", "alloc"]
```
- mark the global allocator
  - add `emballoc` to your dependencies
  - specified the global_allocator: ``
    ```rust 
       #[global_allocator]
       static ALLOCATOR: emballoc::Allocator<4096> = emballoc::Allocator::new();
    ```

### Overview of Actor's Life Cycle
Here represents a simple illustration :

```
actor::create --> ActorRegister --> push to global static Register --> dropped when closed
```

### Changes 
- Switch to static Register instead of lazy_static / spinlocks

recall that the blog os defines a glocal `Writer` with `lazy_static` 
and guarded with `spin::Mutex`
```rust 
lazy_static! {
    pub static ref WRITER: Mutex<Writer> = Mutex::new(Writer {
        column_position: 0,
        color_code: ColorCode::new(Color::Yellow, Color::Black),
        buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
    });
}
```

with crossbus, you 

- don't have to worry about static / lock, or some stuff like that;

- access any type implemented trait `Actor` with global static mutable/sharing reference; 

### Details 

So the dependencies `spin` or `lazy_static` is not necessary here:

```rust 
// define the message
pub struct Msg(String);

// impl the Message trait
impl message::Message for Msg {}

impl Actor for Writer {
    type Message = Msg;

		// normally create the Writer
    fn create(_: &mut Context<Self>) -> Self {
        Writer {
            column_position: 0,
            color_code: ColorCode::new(Color::Yellow, Color::Black),
            buffer: unsafe { &mut *(0xb8000 as *mut Buffer) },
        }
    }

    // this line is not necessary though
    // a syntax must
    //
    // since we dont use an writer instance then
    // call this method
    fn action(&mut self, _: Self::Message, _: &mut Context<Self>) {}
}
```

According to the illustration above, 
once the actor is created, you can access it as glocal static item,
and downcast it to the original type `Writer`, and you don't 
manually lock the `Writer` ( crossbus has assure its exclusively access)
```rust 
// mark the writer's actor id
// used to get writer from `crossbus::Register`
static WRITERID: AtomicUsize = AtomicUsize::new(0);

// if Writer is not created
// then create one
if WRITERID.load(Ordering::Acquire) == 0 {
		let (_, id) = Writer::start();
		WRITERID.store(id, Ordering::Release);
}

// find the actor by id and
Register::get(WRITERID.load(Ordering::Acquire))
		.unwrap()
		// downcast the actor as mutable reference of Writer
		.downcast_mut::<Writer, _>(
				// here pass a closure,
				// writer is a mutable reference
				|writer: &mut Writer| {
						// and use it write string to display
						writer.write_fmt(args).unwrap();
						Ok(())
				},
		);
```

### Results
go the project directory and use `cargo run`, you will see:
![vga-text](../assets/vga-text.png)

### Code
this is the a section of integrating blog os with crossbus,
the full code is at [here](https://github.com/hominee/crossbus/tree/master/examples/blog-os/vga-text) 


