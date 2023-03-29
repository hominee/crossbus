# CrossBus a New Runtime Less Platform Less Actor Computing Model

Hello, Rustaceans!

after months' work on this project, the first version of [crossbus](https://github.com/hominee/crossbus) is out. A Rust library for Platform-less Runtime-less Actor-based Computing, an implementation of Actor Computing Model, with the concept that

- **Runtime-Less**

  crossbus neither provides runtime for downstream app execution, nor accesses the system interface. Going Runtime less doesn't mean execution without runtime, but more precisely, no built-in runtime, but allow any runtime.

- **libstd-free and Bare-Metal Compatible**

  links to no std library, no system libraries, no libc, and a few upstream libraries.

  All crossbus need to run on a bare metal is a allocator. (see [example](https://github.com/hominee/crossbus/tree/master/examples/no-std))

- **Platform-less by Runtime-less**

  bypass the limitation of runtime implementation which often relies on a specific platform or operating system abstraction, crossbus can go platform-less.

- **Future-oriented Routine and Events**

  crossbus defines a set of types and traits to allow asynchronous tasks manipulation, actor communication, message handling, context interaction and etc all are asynchronous. That means you can run a future on a bare metal

- **Real-time Execution Control**

  taking the advantage of the design of future routine, each poll and events alongside can be intercepted for each spawned task during execution, with which more execution control is possible.

Currently crossbus is in its alpha version, all APIs and architecture is not stable yet, and evolves very quickly.

to get a taste of crossbus, you can check out the [examples](https://github.com/hominee/crossbus/tree/master/examples) which demonstrates how to use crossbus in std/ no-std/ web-assembly/ etc.

Feel Free to Contribute to this repository! All issues / bugs-report / feature-request / etc are welcome!

## Possible Questions and Answers

- why create crossbus

  this is the most natural and obvious question, I guess, about crossbus,  
  why not use some frameworks like `actix` `rotor` `go-actor` or `orleans`, they are mature and have rich ecosystem? 
  besides the features mentioned above, crossbus is aimed at extending actor-based computing to a lower level with more compatibility. 

- why split off runtime  

  it could be the most obvious question after the first, most of frameworks 
  built upon some kind of runtime or targeting for certain runtime .
  why crossbus take a different road?

  Imagine a scenario that you want certain functionality of a project,
  but when looking at 
  the document/ source code of the project, it requires a
  specific runtime. and in order to use it the integration either 
  - change my current runtime (change a lot of stuff, not a good idea)
  - set up it as a service and access it via network ( more complex )
  - compile it into dynamic/static library and load/link it ( most we do )
  - split the crate features off the runtime 

  the last option seems better, right? 
  benefiting from this, more limits can be pushed :
  - less dependent on developing platform
  - less code redundancy and complexity
  - more compatible and efficient

- possible application of crossbus

  with the help of powerful actor computing model with lower level and more compatible support, there are some potentially fields:
    - web assembly
    - operating system development
    - embedded development	
    - game development
    - IoT 
    - web infrastructure 
  

- Is it a defeater to shift runtime implements to downstream?

  Shifting the responsibility of runtime implement to user seems overwhelming.
  But most of time, we don't need a elaborated or luxury runtime like tokio or async-std, or in embedded development, there is no such an runtime for us, this is where crossbus make a difference, some lite executor like [futures-executor](https://docs.rs/futures-executor), [async-executor](https://docs.rs/async-executor) etc also work for crossbus.
  Moreover, in practice, it doesn't change the way you use runtime! On the contrary, offers more choices and more freedom to the user with cost of some line of code.

  you don't touch runtime when you build a wasm app, good, just use crossbus directly and don't do that neither. you like third-party's runtime like tokio, okay, just build and pass it before run crossbus. you use it in a lower level or a less compatible environment/ device where there is no runtime available. fine, use [futures-executor](https://docs.rs/futures-executor) or its fine-tuned version will do.

  Last but not least, even a bare-bone [noop-waker executor](https://docs.rs/futures-task/latest/futures_task/fn.noop_waker.html) can do.

- Is crossbus a completeness implement of actor model

  the short answer is almost but not complete. Crossbus implement 
  almost all features. when an actor gets started, it can make local 
  decisions, create more actors, send more messages, and determine 
  how to respond to the next message received.
  some known incompleteness, so far, is runtime isolation.
  crossbus assumes user use a global runtime which specified
  by user to execute tasks when all business logic set up.  

- how to explicitly handle asynchronous task with crossbus

  suppose you wanna run an async function: 
  ```rust no_run
  async fn run() {
  	// do stuff here
  }
  ```
  you can 
  - convert it into [actor future](https://docs.rs/crossbus/latest/crossbus/actor/trait.Future.html) with [Localizer](https://docs.rs/crossbus/latest/crossbus/actor/struct.Localizer.html) then spawn it with [Context::spawn](https://docs.rs/crossbus/latest/crossbus/context/struct.Context.html#method.spawn)
  - or more directly, use [Context::send_future](https://docs.rs/crossbus/latest/crossbus/context/struct.Context.html#method.send_future)

  for a more general type like [`Stream`](https://docs.rs/crossbus/latest/crossbus/stream/trait.Stream.html), [`Message Stream`](https://docs.rs/crossbus/latest/crossbus/message/trait.MStream.html), [`Delayed Timer/ Message`](https://docs.rs/crossbus/latest/crossbus/delayer/trait.Delaying.html) or [`Blocking wait message`](https://docs.rs/crossbus/latest/crossbus/blocker/trait.Blocking.html),
  crossbus has native support for them, you can just follow [the document](https://docs.rs/crossbus) or see [the integration tests](https://github.com/hominee/crossbus/tree/master/tests) as a reference 

lastly, have a good time and enjoy crossbus!
