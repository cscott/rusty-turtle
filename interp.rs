use op::*;
use function::Function;
use module::Module;
use object::*;
use intern::{intern,intern_to_uint};

struct State {
    // main interpreter state.
    parent: Option<~State>, // calling context (another state)
    frame: @mut Object,
    stack: ~[JsVal],
    pc: uint,
    // from bytecode file
    module: @Module,
    function: @Function
}

impl State {
    fn new(parent: Option<~State>, frame: @mut Object,
           module: @Module, function: @Function) -> State {
        State {
            parent: parent,
            frame: frame,
            stack: vec::with_capacity(function.max_stack),
            pc: 0u,
            module: module,
            function: function
        }
    }
}

struct Environment {
    root_map: @mut ObjectMap,
    myObject: @mut Object,
    myArray: @mut Object,
    myFunction: @mut Object,
    myString: @mut Object,
    myNumber: @mut Object,
    myBoolean: @mut Object,
    myTrue: @mut Object,
    myFalse: @mut Object,
    myMath: @mut Object,
    // usefull field descriptors
    fdProto: FieldDesc,
    fdType: FieldDesc,
    fdValue: FieldDesc,
    fdLength: FieldDesc,
    fdParentFrame: FieldDesc,
    fdIsApply: FieldDesc
}

impl Environment {
    pub fn new() -> ~Environment {
        let root_map = @mut ObjectMap::new();
        let fdProto = FieldDesc { name: intern("__proto__"), hidden: false };
        let fdType = FieldDesc { name: intern("type"), hidden: true };
        let fdValue = FieldDesc { name: intern("value"), hidden: true };
        let fdLength = FieldDesc { name: intern("length"), hidden: false };
        let fdParentFrame = FieldDesc { name: intern("parent_frame"), hidden: true };
        let fdIsApply = FieldDesc { name: intern("is_apply"), hidden: true };

        let myObject = Object::new(root_map); // parent of all objects.
        //myObject.get(fdType);
        myObject.set(fdType, JsVal::from_str("object"));

        let myArray = Object::create(root_map, myObject);
        myArray.set(fdType, JsVal::from_str("array"));
        myArray.set(fdLength, JsNumber(0f64));

        let myFunction = Object::create(root_map, myObject);
        myFunction.set(fdType, JsVal::from_str("function"));
        myFunction.set(fdValue, JsUndefined); // allocate space

        let myString = Object::create(root_map, myObject);
        myString.set(fdType, JsVal::from_str("string"));
        //myString.set(fdValue, JsUndefined); // allocate space

        let myNumber = Object::create(root_map, myObject);
        myNumber.set(fdType, JsVal::from_str("number"));

        let myBoolean = Object::create(root_map, myObject);
        myBoolean.set(fdType, JsVal::from_str("boolean"));

        let myTrue = Object::create(root_map, myBoolean);
        myTrue.set(fdValue, JsNumber(1f64));

        let myFalse = Object::create(root_map, myBoolean);
        myFalse.set(fdValue, JsNumber(0f64));

        let myMath = Object::create(root_map, myObject);

        ~Environment {
            root_map: root_map,
            myObject: myObject,
            myArray: myArray,
            myFunction: myFunction,
            myString: myString,
            myNumber: myNumber,
            myBoolean: myBoolean,
            myTrue: myTrue,
            myFalse: myFalse,
            myMath: myMath,
            fdProto: fdProto,
            fdType: fdType,
            fdValue: fdValue,
            fdLength: fdLength,
            fdParentFrame: fdParentFrame,
            fdIsApply: fdIsApply
        }
    }

    fn add_native_func(&self, frame : @mut Object,
                       obj : @mut Object, desc: FieldDesc,
                       f : NativeFunction) -> @mut Object {
        let my_func = Object::create(self.root_map, self.myFunction);
        my_func.set(self.fdParentFrame, JsObject(frame));
        my_func.set(self.fdValue, JsNativeFunction(f));
        obj.set(desc, JsObject(my_func));
        my_func
    }

    fn add_native_func_str(&self, frame: @mut Object, obj: @mut Object,
                           name: &str, f: NativeFunction) -> @mut Object {
        self.add_native_func(frame, obj, FieldDesc {
            name: intern(name), hidden: false
        }, f)
    }

    /* note that we make a copy of self when this function is called
       (pass by value) which allows us to access self from stack closures
       when we register native functions below. */
    fn make_top_level_frame(self, this : JsVal, arguments: &[JsVal]) -> @mut Object {
        let frame = Object::new(self.root_map); // "Object.create(null)"

        // set up 'this' and 'arguments'
        frame.set(FieldDesc { name: intern("this"), hidden: false }, this);
        let my_arguments = self.arrayCreate(arguments);
        frame.set(FieldDesc { name: intern("arguments"), hidden: false },
                  my_arguments);

        // constructors
        let fdPrototype = FieldDesc { name:intern("prototype"), hidden:false };

        let mkConstructor = |name,proto| {
            let cons = Object::create(self.root_map, self.myFunction);
            cons.set(fdPrototype, JsObject(proto));
            frame.set(FieldDesc { name: intern(name), hidden: false },
                      JsObject(cons));
            cons
        };

        let myObjectCons = mkConstructor("Object", self.myObject);
        mkConstructor("Array", self.myArray);
        mkConstructor("Function", self.myFunction);
        mkConstructor("Boolean", self.myBoolean);
        mkConstructor("String", self.myString);
        mkConstructor("Number", self.myNumber);

        frame.set(FieldDesc { name: intern("Math"), hidden: false },
                  JsObject(self.myMath));

        // support for console.log
        let myConsole = Object::create(self.root_map, self.myObject);
        frame.set(FieldDesc { name: intern("console"), hidden: false },
                  JsObject(myConsole));

        // helper function
        let getarg: @fn(&[JsVal], uint)->JsVal = |args, i| {
            if args.len() > i { args[i] } else { JsUndefined }
        };

        // native functions
        do self.add_native_func_str(frame, myConsole, "log") |_this, args| {
            let sargs = do vec::map_consume(args) |val| { val.to_str() };
            io::println(str::connect(sargs, " "));
            JsUndefined
        };
        do self.add_native_func_str(frame, self.myObject, "hasOwnProperty")
            |this, args| {
            let prop = FieldDesc {
                name: intern(self.toString(getarg(args, 0))),
                hidden: false
            };
            let rv = match(this) {
                JsObject(obj) => obj.contains_simple(prop),
                JsBool(b) =>
                (if b { self.myTrue } else { self.myFalse })
                .contains_simple(prop),
                JsString(utf16) => {
                    if self.fdLength==prop {
                        true
                    } else {
                        match intern_to_uint(prop.name) {
                            Some(n) if n < utf16.len() => true,
                            _ => false
                        }
                    }
                },
                JsNumber(_) => false,
                JsUndefined | JsNull => fail!("TypeError"), // XXX should throw
                _ => fail!()
            };
            JsBool(rv)
        };
        do self.add_native_func_str(frame, myObjectCons, "create")
            |_this, args| {
            let rv = match getarg(args, 0) {
                JsObject(obj) => Object::create(self.root_map, obj),
                JsNull => Object::new(self.root_map),
                _ => fail!("TypeError") // XXX should throw
            };
            JsObject(rv)
        };

        // XXX: We're not quite handling the "this" argument correctly.
        // According to:
        // https://developer.mozilla.org/en/JavaScript/Reference/Global_Objects/Function/call
        // "If thisArg is null or undefined, this will be the global
        // object. Otherwise, this will be equal to Object(thisArg)
        // (which is thisArg if thisArg is already an object, or a
        // String, Boolean, or Number if thisArg is a primitive value
        // of the corresponding type)."
        // this is disallowed in ES-5 strict mode; throws an exception instead
        //  http://ejohn.org/blog/ecmascript-5-strict-mode-json-and-more/
        do self.add_native_func_str(frame, self.myFunction, "call")
            |this, args| {
            // push arguments on stack and use 'invoke' bytecode op.
            // arg #0 is the function itself ('this')
            // arg #1 is 'this' (for the invoked function)
            // arg #2-#n are rest of arguments
            self.arrayCreate(~[this] +
                             if args.len()>0 {args} else {~[JsUndefined]})
        }.set(self.fdIsApply, JsBool(true));

        do self.add_native_func_str(frame, self.myFunction, "apply")
            |this, args| {
            // push arguments on stack and use 'invoke' bytecode op.
            // arg #0 is the function itself ('this')
            // arg #1 is 'this' in the invoked function
            // arg #2 is rest of arguments, as array
            let mut nargs : ~[JsVal] = ~[ this ];
            nargs.push( getarg(args, 0) );
            if args.len() > 1 {
                for self.arrayEach( args[1] ) |v| {
                    nargs.push(v)
                }
            }
            self.arrayCreate(nargs) // this is the natural order
        }.set(self.fdIsApply, JsBool(true));

        frame
    }

    pub fn toString(&self, val: JsVal) -> ~str {
        // xxx invoke toString?
        val.to_str()
    }
    pub fn get_slot(&self, obj: JsVal, name: JsVal) -> JsVal {
        let desc = FieldDesc {
            name: intern(match name {
                JsString(utf16) => str::from_utf16(utf16),
                _ => self.toString(name)
            }),
            hidden: false
        };
        self.get_slot_fd(obj, desc)
    }
    pub fn get_slot_fd(&self, obj: JsVal, desc: FieldDesc) -> JsVal {
        match obj {
            JsString(utf16) => {
                if desc == self.fdProto {
                    JsObject(self.myString)
                } else if desc == self.fdLength {
                    JsNumber(utf16.len() as f64)
                } else {
                    match intern_to_uint(desc.name) {
                        Some(n) => if (n < utf16.len()) {
                            io::println("WARNING: accessing string char by index");
                            JsString(@[utf16[n]])
                        } else {
                            JsUndefined
                        },
                        None => {
                            self.myString.get(desc)
                        }
                    }
                }
            },
            JsBool(b) => {
                if desc == self.fdProto {
                    JsObject(self.myBoolean)
                } else {
                    (if b { self.myTrue } else { self.myFalse }).get(desc)
                }
            },
            JsNumber(_) => {
                if desc == self.fdProto {
                    JsObject(self.myNumber)
                } else {
                    self.myNumber.get(desc)
                }
            },
            JsObject(o) => {
                // XXX add basic typed array support here
                o.get(desc) // xxx prototype chains can't include special types
            },
            JsUndefined => {
                fail!("dereference of undefined; should throw exception");
            },
            JsNull => {
                fail!("dereference of null; should throw exception");
            },
            _ => {
                fail!("dereference of unexpected type!");
            }
        }
    }
    pub fn set_slot(&self, obj: JsVal, name: JsVal, nval: JsVal) {
        let desc = FieldDesc {
            name: intern(match name {
                JsString(utf16) => str::from_utf16(utf16),
                _ => self.toString(name)
            }),
            hidden: false
        };
        match obj {
            JsObject(obj) => match obj.get(self.fdType).to_str() {
                ~"array" => {
                    // handle array sets specially: they update the length field
                    if (desc == self.fdLength) {
                        // sanity-check the new length.
                        let nlen = match nval.to_uint() {
                            Some(n) => n,
                            // XXX this should throw RangeError
                            _ => fail!(fmt!("RangeError %?", nval))
                        };
                        // truncate the array
                        let mut olen = match obj.get(desc) {
                            JsNumber(n) => n as uint,
                            _ => fail!()
                        };
                        while olen > nlen {
                            // XXX should delete field
                            let name = intern((olen-1).to_str());
                            obj.set(FieldDesc { name:name, hidden:false },
                                    JsUndefined);
                            olen -= 1;
                        }
                        obj.set(desc, JsNumber(nlen as f64));
                    } else {
                        match intern_to_uint(desc.name) {
                            Some(n) => {
                                let len = match obj.get(self.fdLength) {
                                    JsNumber(n) => n as uint,
                                    _ => fail!()
                                };
                                if (n >= len) {
                                    obj.set(self.fdLength,
                                            JsNumber((n+1) as f64));
                                }
                                obj.set(desc, nval);
                            },
                            None => {
                                obj.set(desc, nval);
                            }
                        }
                    }
                },
                ~"object" if obj.contains(FieldDesc{name:intern("buffer"),
                                                    hidden:true}) => {
                    // very basic TypedArray support.
                    fail!("unimplemented");
                },
                _ => { obj.set(desc, nval); }
            },
            JsBool(b) => {
                // handle writes to booleans (not supported in standard js)
                (if b { self.myTrue } else { self.myFalse }).set(desc, nval);
            },
            JsNumber(_) | JsString(_) => {
                /* ignore write to field of primitive value */
            },
            JsUndefined | JsNull => {
                // XXX should throw TypeError
                fail!(fmt!("TypeError: Cannot set property %? of %?",name,obj));
            },
            JsFunctionCode(_) | JsNativeFunction(_) => {
                fail!(fmt!("%? shouldn't escape!", obj));
            }
        }
    }

    pub fn arrayCreate(&self, elements: &[JsVal]) -> JsVal {
        let arr = Object::create(self.root_map, self.myArray);
        arr.set(self.fdLength, JsNumber(elements.len() as f64));
        for elements.eachi |i, v| {
            // xxx converting array indexes to string is a bit of fail.
            arr.set(FieldDesc { name: intern(i.to_str()), hidden: false },
                    *v);
        }
        JsObject(arr)
    }

    pub fn arrayEach(&self, a: JsVal, f: &fn(JsVal) -> bool) -> bool {
        match a {
            JsObject(arr) => {
                let mut i = 0u;
                let mut len = arr.get(self.fdLength).to_uint()
                    .expect("no length");
                while i < len {
                    let v = self.get_slot(a, JsNumber(i as f64));
                    if !f(v) { return false; }
                    i += 1;
                    // this next is not strictly necessary for most cases,
                    // but it makes the iterator more robust
                    len = arr.get(self.fdLength).to_uint()
                        .expect("length disappeared");
                }
                true
            },
            _ => fail!()
        }
    }

    priv fn invoke(&self, mut state: ~State, arg1: uint) -> ~State {
        // collect arguments
        let mut native_args : ~[JsVal] = vec::with_capacity(arg1);
        for uint::range(0, arg1) |_| {
            native_args.push(state.stack.pop());
        }
        vec::reverse(native_args);
        // collect 'this'
        let my_this = state.stack.pop();
        // get function object
        let func = match state.stack.pop() {
            JsObject(obj) => obj,
            _ => {
                // xxx throw wrapped TypeError
                fail!(fmt!("Not a function at %u", state.pc));
            }
        };
        // assert that func is a function
        match func.get(self.fdType) {
            JsString(utf16) if "function"==str::from_utf16(utf16) => {
                /* okay! */
            },
            _ => {
                // xxx throw wrapped TypeError
                fail!(fmt!("Not a function at %u", state.pc));
            }
        };
        match func.get(self.fdValue) {
            JsNativeFunction(f) => {
                // "native code"
                // build proper native arguments array
                let rv = f(my_this, native_args);
                // handle "apply-like" natives
                match func.get(self.fdIsApply) {
                    JsBool(true) => {
                        let mut nArgs = 0u;
                        for self.arrayEach(rv) |v| {
                            state.stack.push(v);
                            nArgs += 1;
                        }
                        return self.invoke(state, nArgs-2);
                    },
                    _ => {
                        state.stack.push(rv);
                        return state;
                    }
                };
            },
            JsFunctionCode(f) => {
                // create new frame
                let parent_frame = match func.get(self.fdParentFrame) {
                    JsObject(obj) => obj,
                    _ => fail!()
                };
                let nframe = Object::create(self.root_map,
                                            parent_frame);
                nframe.set(FieldDesc {
                    name: intern("this"), hidden: false
                }, my_this);
                nframe.set(FieldDesc {
                    name: intern("arguments"), hidden: false
                }, self.arrayCreate(native_args));
                // construct new child state
                return ~State::new(Some(state), nframe,
                                   f.module, f.function);
            },
            _ => { fail!("bad function object"); }
        };
    }

    priv fn unary(&self, state: &mut State, uop: &fn(arg: JsVal) -> JsVal) {
        let arg = state.stack.pop();
        let rv = uop(arg);
        state.stack.push(rv);
    }

    priv fn binary(&self, state: &mut State, bop: &fn(left: JsVal, right: JsVal) -> JsVal) {
        let right = state.stack.pop();
        let left = state.stack.pop();
        let rv = bop(left, right);
        state.stack.push(rv);
    }

    pub fn interpret(&self, module: @Module, func_id: uint) -> JsVal {
        let frame = self.make_top_level_frame(JsNull, ~[]);
        let function = module.functions[func_id];
        let top = ~State::new(None, frame, module, function);
        let mut state = ~State::new(Some(top), frame, module, function);
        while state.parent.is_some() /* wait for state == top */ {
            state = self.interpret_one(state);
        }
        state.stack.pop()
    }

    pub fn interpret_one(&self, mut state: ~State) -> ~State {
        //io::println(fmt!("pc %u stack %?", state.pc, state.stack.len()));
        let op = Op::new_from_uint(state.function.bytecode[state.pc]);
        state.pc += 1;
        let arg1;
        match op.args() {
            0 => { arg1 = 0; }
            1 => { arg1 = state.function.bytecode[state.pc]; state.pc +=1; }
            _ => fail!()
        }
        match op {
            Op_push_frame => {
                state.stack.push(JsObject(state.frame));
            },
            Op_push_literal => {
                state.stack.push(match state.module.literals[arg1] {
                    JsBool(b) =>
                        JsObject(if b { self.myTrue } else { self.myFalse }),
                    other => other
                });
            },
            Op_new_object => {
                let obj = Object::create(self.root_map, self.myObject);
                state.stack.push(JsObject(obj));
            },
            Op_new_array => {
                let na = Object::create(self.root_map, self.myArray);
                na.set(self.fdLength, JsNumber(0f64));
                state.stack.push(JsObject(na));
            },
            Op_new_function => {
                let function = state.module.functions[arg1];
                let f = Object::create(self.root_map, self.myFunction);
                // hidden fields of function object
                f.set(self.fdParentFrame, JsObject(state.frame));
                f.set(self.fdValue, JsFunctionCode(@InterpretedFunction {
                    module: state.module,
                    function: function
                }));
                // user-visible fields
                f.set(FieldDesc{name:intern("name"),hidden:false},
                      match function.name {
                          Some(copy s) => JsVal::from_str(s),
                          None => JsUndefined
                      });
                f.set(self.fdLength, JsNumber(function.nargs as f64));
                state.stack.push(JsObject(f));
            },
            Op_get_slot_direct => {
                let obj = state.stack.pop();
                let name = state.module.literals[arg1];
                state.stack.push(self.get_slot(obj, name));
            },
            Op_get_slot_direct_check => {
                let obj = state.stack.pop();
                let name = state.module.literals[arg1];
                let result = self.get_slot(obj, name);
                match result {
                    JsObject(_) => {/* okay! */},
                    _ => {
                        // warn about unimplemented (probably library) functions
                        io::println(fmt!("Failing lookup of method %?",
                                         name.to_str()));
                    }
                }
                state.stack.push(result);
            },
            Op_get_slot_indirect => {
                let name = state.stack.pop();
                let obj = state.stack.pop();
                state.stack.push(self.get_slot(obj, name));
            },
            Op_set_slot_direct => {
                let nval = state.stack.pop();
                let name = state.module.literals[arg1];
                let obj = state.stack.pop();
                self.set_slot(obj, name, nval);
            },
            Op_set_slot_indirect => {
                let nval = state.stack.pop();
                let name = state.stack.pop();
                let obj = state.stack.pop();
                self.set_slot(obj, name, nval);
            },
            Op_invoke => {
                state = self.invoke(state, arg1);
            },
            Op_return => {
                let retval = state.stack.pop();
                // go up to the parent state
                // use pattern matching to work around a limitation of the
                // type system; ideally this should work:
                //state = state.parent.expect("return from top of stack");
                let ~State { parent: parent, _ } = state;
                state = parent.expect("return from top of stack");
                state.stack.push(retval);
                // continue in parent state
            },

            // branches
            Op_jmp => {
                state.pc = arg1;
            },
            Op_jmp_unless => {
                let cond = state.stack.pop();
                // also has to convert to boolean; see Op_un_not
                match cond {
                    JsBool(b) => {
                        if !b { state.pc = arg1; }
                    },
                    JsString(utf16) => {
                        if utf16.is_empty() { state.pc = arg1; }
                    },
                    JsNumber(n) => {
                        if n==0f64 { state.pc = arg1; }
                    },
                    JsObject(_) => { /* no op */ },
                    JsUndefined | JsNull => {
                        state.pc = arg1;
                    },
                    _ => fail!(fmt!("bad argument to jmp_unless: %?", cond))
                };
            },

            // stack manipulation
            Op_pop => {
                state.stack.pop();
            },
            Op_dup => {
                let top = *(state.stack.last());
                state.stack.push(top);
            },
            Op_2dup => {
                let len = state.stack.len();
                let top = state.stack[len-1];
                let nxt = state.stack[len-2];
                state.stack.push(nxt);
                state.stack.push(top);
            },
            Op_over => {
                let top = state.stack.pop();
                let nxt = state.stack.pop();
                state.stack.push(top);
                state.stack.push(nxt);
                state.stack.push(top);
            },
            Op_over2 => {
                let top = state.stack.pop();
                let nx1 = state.stack.pop();
                let nx2 = state.stack.pop();
                state.stack.push(top);
                state.stack.push(nx2);
                state.stack.push(nx1);
                state.stack.push(top);
            },
            Op_swap => {
                let top = state.stack.pop();
                let nxt = state.stack.pop();
                state.stack.push(top);
                state.stack.push(nxt);
            },

            // unary operators
            Op_un_not => do self.unary(state) |arg| {
                match arg {
                    JsBool(b) => JsBool(!b),
                    JsString(utf16) => JsBool(utf16.is_empty()),
                    JsNumber(n) => JsBool(n==0f64),
                    JsObject(_) => JsBool(false),
                    JsUndefined | JsNull => JsBool(true),
                    _ => fail!(fmt!("unimplemented case for not: %?", arg))
                }
            },
            Op_un_minus => do self.unary(state) |arg| {
                match arg {
                    JsNumber(n) => JsNumber(-n),
                    _ => fail!(fmt!("unimplemented case for minus: %?", arg))
                }
            },
            Op_un_typeof => do self.unary(state) |arg| {
                match arg {
                    JsUndefined => JsVal::from_str("undefined"),
                    JsNull => JsVal::from_str("object"),
                    _ => {
                        let ty = self.get_slot_fd(arg, self.fdType);
                        match ty.to_str() {
                            ~"array" => {
                                /* weird javascript misfeature */
                                JsVal::from_str("object")
                            },
                            _ => ty
                        }
                    }
                }
            },

            // binary operators
            Op_bi_eq => do self.binary(state) |left, right| {
                match (left, right) {
                    (JsNumber(l), JsNumber(r)) => JsBool(l == r),
                    (JsString(l), JsString(r)) => JsBool(l == r),
                    _ => fail!(fmt!("unimplemented case for bi_eq: %? %?", left, right))
                }
            },
            Op_bi_gt => do self.binary(state) |left, right| {
                match (left, right) {
                    (JsNumber(l), JsNumber(r)) => JsBool(l > r),
                    _ => fail!(fmt!("unimplemented case for bi_gt: %? %?", left, right))
                }
            },
            Op_bi_gte => do self.binary(state) |left, right| {
                match (left, right) {
                    (JsNumber(l), JsNumber(r)) => JsBool(l >= r),
                    _ => fail!(fmt!("unimplemented case for bi_gte: %? %?", left, right))
                }
            },
            Op_bi_add => do self.binary(state) |left, right| {
                match (left, right) {
                    (JsNumber(l), JsNumber(r)) => JsNumber(l + r),
                    _ => fail!(fmt!("unimplemented case for bi_add: %? %?", left, right))
                }
            },
            Op_bi_sub => do self.binary(state) |left, right| {
                match (left, right) {
                    (JsNumber(l), JsNumber(r)) => JsNumber(l - r),
                    _ => fail!(fmt!("unimplemented case for bi_sub: %? %?", left, right))
                }
            },
            Op_bi_mul => do self.binary(state) |left, right| {
                match (left, right) {
                    (JsNumber(l), JsNumber(r)) => JsNumber(l * r),
                    _ => fail!(fmt!("unimplemented case for bi_mul: %? %?", left, right))
                }
            },
            Op_bi_div => do self.binary(state) |left, right| {
                match (left, right) {
                    (JsNumber(l), JsNumber(r)) => JsNumber(l / r),
                    _ => fail!(fmt!("unimplemented case for bi_div: %? %?", left, right))
                }
            }
        }
        state
    }
}
