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
            stack: ~[],
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
    fdLength: FieldDesc
}

impl Environment {
    pub fn new() -> ~Environment {
        let root_map = @mut ObjectMap::new();
        let fdProto = FieldDesc { name: intern("__proto__"), hidden: false };
        let fdType = FieldDesc { name: intern("type"), hidden: true };
        let fdValue = FieldDesc { name: intern("value"), hidden: true };
        let fdLength = FieldDesc { name: intern("length"), hidden: false };

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
            fdLength: fdLength
        }
    }

    fn add_native_func(&self, frame : @mut Object,
                       obj : @mut Object, desc: FieldDesc,
                       f : NativeFunction) -> @mut Object {
        let my_func = Object::create(self.root_map, self.myFunction);
        my_func.set(FieldDesc { name: intern("parent_frame"), hidden: true },
                    JsObject(frame));
        my_func.set(self.fdValue, JsNativeFunction(f));
        /*
        my_func.set(FieldDesc { name: intern("is_apply"), hidden: true },
                    JsObject(if is_apply {self.myTrue} else {self.myFalse}));
        */
        obj.set(desc, JsObject(my_func));
        my_func
    }

    fn make_top_level_frame(&self, this : JsVal, arguments: &[JsVal]) -> @mut Object {
        let frame = Object::new(self.root_map); // "Object.create(null)"

        // set up 'this' and 'arguments'
        frame.set(FieldDesc { name: intern("this"), hidden: false }, this);
        let myArgs = Object::create(self.root_map, self.myArray);
        myArgs.set(self.fdLength, JsNumber(arguments.len() as f64));
        for arguments.eachi |i, v| {
            // xxx converting array indexes to string is a bit of fail.
            myArgs.set(FieldDesc { name: intern(i.to_str()), hidden: false },
                       *v);
        }
        frame.set(FieldDesc { name: intern("arguments"), hidden: false },
                  JsObject(myArgs));

        // constructors
        let fdPrototype = FieldDesc { name:intern("prototype"), hidden:false };

        let mkConstructor = |name,proto| {
            let cons = Object::create(self.root_map, self.myFunction);
            cons.set(fdPrototype, JsObject(proto));
            frame.set(FieldDesc { name: intern(name), hidden: false },
                      JsObject(cons));
        };

        mkConstructor("Object", self.myObject);
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

        // native functions
        do self.add_native_func(frame, myConsole,
                                FieldDesc{ name: intern("log"), hidden: false })
            |_this, args| {
            let sargs = do vec::map_consume(args) |val| { val.to_str() };
            io::println(str::connect(sargs, " "));
            JsUndefined
        };

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
            Op_invoke => {
                // collect arguments
                let myArgs = Object::create(self.root_map, self.myArray);
                myArgs.set(self.fdLength, JsNumber(arg1 as f64));
                let mut i = arg1;
                while i > 0 {
                    let name = intern((i-1).to_str());
                    myArgs.set(FieldDesc { name:name, hidden:false },
                               state.stack.pop());
                    i -= 1;
                }
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
                let rv = match func.get(self.fdValue) {
                    JsNativeFunction(f) => {
                        // "native code"
                        // build proper native arguments array
                        let mut native_args : ~[JsVal] =
                            vec::with_capacity(arg1);
                        i = 0;
                        while i < arg1 {
                            let desc = FieldDesc {
                                name: intern(i.to_str()),
                                hidden: false
                            };
                            native_args.push(myArgs.get(desc));
                            i += 1;
                        }
                        // XXX handle "apply-like" natives
                        f(my_this, native_args)
                    },
                    JsFunctionCode(_f) => {
                        fail!("functions not implemented"); //XXX
                    },
                    _ => { fail!("bad function object"); }
                };
                state.stack.push(rv);
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

            // stack manipulation
            Op_pop => {
                state.stack.pop();
            },
            Op_dup => {
                let top = *(state.stack.last());
                state.stack.push(top);
            },
            Op_swap => {
                let top = state.stack.pop();
                let nxt = state.stack.pop();
                state.stack.push(top);
                state.stack.push(nxt);
            },
            _ => fail!() // unimplemented
        }
        state
    }
}
