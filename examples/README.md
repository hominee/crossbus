## Intro

Some examples that use crossbus in std, no-std or web-assembly
development.

There are three types of small demos presented :
- `ping`: send some numbers to actor and get their sum. 
- `fibonacci`: send the _n_-th  term of the Fibonacci sequence.
- `ring`: loop a message in a _n_ actor-linked cycle for _m_ rounds.

And the [no-std](https://github.com/hominee/crossbus/tree/master/examples/no-std) folder is a no-std version of the above three projects.

The [wasm32](https://github.com/hominee/crossbus/tree/master/examples/wasm32) folder is a web-assembly version of the above three projects.


## How to use 

Since running these examples require some features enabled. 
You can manually pass flags when running them, but 
it it advised to run it with bash file [runner](https://github.com/hominee/crossbus/blob/master/runner)
which provides some convenient shortcuts for 
- running the examples via `./runner demo`
- checking the project  via `./runner check`
- testing the integration tests via `./runner test`

or you can just get help info via 
```bash
./runner help 
```

Before running the examples, some prerequisites are required,
eg. trunk is used for wasm32 example.

and you should review its readme file beforehand.
