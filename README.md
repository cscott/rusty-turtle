# rusty-turtle

`rusty-turtle` is an implementation of
[TurtleScript](https://github.com/cscott/turtlescript) in
[Rust](http://www.rust-lang.org/).  TurtleScript is a syntactic
(but not semantic) subset of JavaScript, originally created for
the One Laptop per Child project.

## Build, Install, and Run

Not hard... just start by building the latest (28-May-2013) version of rust.
No problem, right?  After that,
```
$ make
```
will create an executable named `main`.  To run a TurtleScript
[REPL](http://en.wikipedia.org/wiki/Read%E2%80%93eval%E2%80%93print_loop):
```
$ ./main
> 2+3
5
> var fact = function(x) { return (x<2) ? x : (x * fact(x-1)) ; };
undefined
> fact(42)
1405006117752880268066222604204040608686664282428002
>
```
Use Control-D (or Control-C) to exit the REPL.  You can also evaluate entire
TurtleScript scripts by passing the name on the command line:
```
$ ./main foo.js
```

## Testing
There are quite a few unit tests built into `rusty-turtle` (although never
enough!).  You can build and run them with `make test`.  See the end of
`interp.rs` for a set of script-based tests, which you could manually
reproduce in the REPL (if you were so inclined).

## Design
`rusty-turtle` is a simple interpreter for the bytecode emitted by
`bcompile.js` from the TurtleScript project.  It is heavily based on
`binterp.js` from that project, which is a TurtleScript interpreter written
in TurtleScript.  The `startup.rs` file contains the bytecode for the
TurtleScript standard library implementation (from `binterp.js`) as
well as the tokenizer, parser, and bytecode compiler itself (emitted
by `write-rust-bytecode.js` in the TurtleScript project).  This allows
the `rusty-turtle` REPL to parse and compile the expressions you type
at it into modules which it can interpret.

The interpreter is not particularly fast, however it could become so.
The object model used associates an object map with every object; this
map gives the position of all fields in the object.  One of the ideas
behind `rusty-turtle` was to explore the JavaScript/Rust interaction
model; I wanted to see if one could use "native rust" data structures
from JavaScript by simply providing an appropriate object map, and
vice-versa.  In fact, the vice-versa case was more interesting;
since I'm one of the maintainers of
[Domino](https://github.com/fgnass/domino), which is a fork of
Mozilla's [dom.js](https://github.com/andreasgal/dom.js) project,
I was interested in exploring whether the native javascript
data structure used by domino/dom.js could in fact be accessed
directly by rust (since [Servo](https://github.com/mozilla/servo)
uses hand-coded JS bindings to a native Rust data structure instead).

For a fast(er) interpreter, the field lookup in the object map would
happen only the first time a method was executed/interpreted; future
uses would use a direct dereference of the appropriate field in the
object (so long as the object map for the value remained the same).

## Other research ideas

I've already described dom.js/servo as an interesting experiment.  Other
research ideas which could be pursued with `rusty-turtle`:

1. Concurrency/parallelism exploration.  The most obvious map to Rust's
current concurrency mechanisms would be a web workers-style message-passing
interface.  Each TurtleScript "worker" would have its own Rust task and
@-heap, and use (wrapped) Rust message pipes to communicate.

    But a more interesting exploration would use data sharing and fork/join
parallelism.  This could be pursued along with the experimental
fork/join support in Rust, or it could be emulated using remote objects.

    Alternatively, speculative/transactional execution could be explored.
A typical web page begins with a handful of `<script>` tags, often
loading independent libraries.  One could imagine executing all those
scripts in parallel, pausing or aborting execution only if a sequential
dependency was violated.

2. Export Rust's llvm interface to TurtleScript code, then write a TurtleScript
JIT in TurtleScript, using the Rust data structure for objects in memory.

3. Tweak TurtleScript/`rusty-turtle` to parse/interpret
[asm.js](http://asmjs.org/); then write a bytecode-to-asm.js compiler
for TurtleScript.  The object map, layout and field lookup/access code
would probably be moved into TurtleScript, such that the heap is
accessed via a Typed Array.

4. Optimization via partial evaluation.  The interpreter still needs
to do a significant amount of type matching and virtual dispatch.
Much of this could be simplified via partial evaluation.  That is,
the first time a method is executed, various constants and near-constants
(such as the object map corresponding to the method arguments) would
be tracked and a specialized version of the method compiled which
propagates all the constants through the method, including field offsets
and method dispatch targets.

## License

TurtleScript and `rusty-turtle` are (c) 2010-2013 C. Scott Ananian and
licensed under the terms of the GNU GPL v2.
