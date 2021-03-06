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
    fdIsApply: FieldDesc,
    fdDefaultValue: FieldDesc
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
        let fdDefaultValue = FieldDesc { name: intern("DefaultValue"), hidden: true };

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
            fdIsApply: fdIsApply,
            fdDefaultValue: fdDefaultValue
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
    pub fn make_top_level_frame(self, this : JsVal, arguments: &[JsVal]) -> @mut Object {
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
        let myBooleanCons = mkConstructor("Boolean", self.myBoolean);
        let myStringCons = mkConstructor("String", self.myString);
        mkConstructor("Number", self.myNumber);

        frame.set(FieldDesc { name: intern("Math"), hidden: false },
                  JsObject(self.myMath));

        // helper function
        let getarg: @fn(&[JsVal], uint)->JsVal = |args, i| {
            if args.len() > i { args[i] } else { JsUndefined }
        };

        // Boolean called as function
        myBooleanCons.set(self.fdParentFrame, JsObject(frame));
        myBooleanCons.set(self.fdValue, JsNativeFunction(|_this, args| {
            JsBool(self.toBoolean(getarg(args, 0)))
        }));

        // support for console.log
        let myConsole = Object::create(self.root_map, self.myObject);
        frame.set(FieldDesc { name: intern("console"), hidden: false },
                  JsObject(myConsole));

        // native functions
        do self.add_native_func_str(frame, myConsole, "log") |_this, args| {
            let sargs = do vec::map_consume(args) |val| { val.to_str() };
            io::println(str::connect(sargs, " "));
            JsUndefined
        };
        let opts = do self.add_native_func_str(frame, self.myObject, "toString")
            |this, _args| {
            let _o = self.toObject(this);
            // XXX fetch the [[Class]] internal property of o
            JsVal::from_str("[object]")
        };
        do self.add_native_func_str(frame, self.myArray, "toString")
            |this, _args| {
            let o = self.toObject(this);
            let mut func = o.get(FieldDesc{name:intern("join"),hidden:false});
            if !self.isCallable(func) {
                func = JsObject(opts);
            }
            self.interpret_function(func, JsObject(o), ~[])
        };
        do self.add_native_func_str(frame, self.myObject, "valueOf")
            |this, _args| {
            let o = self.toObject(this);
            // XXX host object support?
            JsObject(o)
        };
        do self.add_native_func(frame, self.myObject, self.fdDefaultValue)
            |this, args| {
            let isDate = false; // XXX fix when we support date objects
            let rawhint = match getarg(args, 0) {
                JsString(utf16) => Some(str::from_utf16(utf16)),
                _ => None
            };
            let hint = match rawhint {
                Some(~"String") => "String",
                Some(~"Number") => "Number",
                _ if !isDate => "Number",
                _ => "String"
            };
            let toString = self.get_slot(this, JsVal::from_str("toString"));
            let valueOf = self.get_slot(this, JsVal::from_str("valueOf"));
            let first, second;
            if "String"==hint {
                first = toString; second = valueOf;
            } else {
                first = valueOf; second = toString;
            }
            let mut rv : Option<JsVal> = None;
            if self.isCallable(first) {
                let rv1 = self.interpret_function(first, this, ~[]);
                match rv1 {
                    JsObject(_) => { /* not primitive, fall through */ },
                    _ => { rv = Some(rv1); }
                }
            }
            if rv.is_none() && self.isCallable(second) {
                let rv2 = self.interpret_function(second, this, ~[]);
                match rv2 {
                    JsObject(_) => { /* not primitive, fall through */ },
                    _ => { rv = Some(rv2); }
                }
            }
            match rv {
                None => fail!("TypeError"), // XXX throw
                Some(rv3) => rv3
            }
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
        do self.add_native_func_str(frame, self.myBoolean, "valueOf")
            |_this, _args| {
            match this {
                JsBool(_) => this,
                JsObject(_) => fail!("Boolean.valueOf() unimplemented"),
                _ => fail!(fmt!("TypeError: %s", this.to_str()))
            }
        };
        do self.add_native_func_str(frame, frame, "isNaN") |_this, args| {
            JsBool(self.toNumber(getarg(args, 0)).is_NaN())
        };
        do self.add_native_func_str(frame, frame, "isFinite") |_this, args| {
            JsBool(self.toNumber(getarg(args, 0)).is_finite())
        };
        do self.add_native_func_str(frame, frame, "parseInt") |_this, args| {
            let number = getarg(args, 0);
            let radix = match getarg(args, 1) {
                // falsy values become radix 10
                JsBool(false) | JsString([]) | JsObject(_) |
                JsUndefined | JsNull => 10u,
                r => match self.toNumber(r) {
                    n if !n.is_finite() => 10u,
                    n if n<2f64 || n>=37f64 => 0, // aka bail
                    n => n as uint
                }
            };
            let rv = match number {
                JsString(utf16) if radix!=0 => {
                    // XXX what about numbers larger than 2^32?
                    // XXX parseInt(' 10z ', 16) = 16, so we seem to trim
                    //     non-digit chars from the right.
                    let s = str::from_utf16(utf16);
                    match int::from_str_radix(s.trim(), radix) {
                        Some(n) => n as f64,
                        None => f64::NaN
                    }
                },
                JsNumber(n) if radix!=0 => {
                    // this is weird, but seems to match EcmaScript
                    match int::from_str_radix(n.to_str(), radix) {
                        Some(n) => n as f64,
                        None => f64::NaN
                    }
                },
                _ => f64::NaN
            };
            JsNumber(rv)
        };
        do self.add_native_func_str(frame, frame, "now")
            |_this, _args| {
            fail!("now() unimplemented");
        };
        do self.add_native_func_str(frame, self.myString, "charAt")
            |this, args| {
            let idx = match self.toNumber(getarg(args, 0)) {
                n if n.is_NaN() => 0i, // strange
                n => n as int
            };
            match this {
                JsString(utf16) => {
                    if 0 <= idx && idx < (utf16.len() as int) {
                        JsString(@[utf16[idx]])
                    } else {
                        JsString(@[])
                    }
                },
                // XXX probably should support String('abc'), which is an
                // Object whose prototype is a String...
                _ => fail!("charAt called on a non-string")
            }
        };
        do self.add_native_func_str(frame, self.myString, "charCodeAt")
            |this, args| {
            let idx = match self.toNumber(getarg(args, 0)) {
                n if n.is_NaN() => 0i, // strange
                n => n as int
            };
            let rv = match this {
                JsString(utf16) => {
                    if 0 <= idx && idx < (utf16.len() as int) {
                        utf16[idx] as f64
                    } else {
                        f64::NaN
                    }
                },
                // XXX probably should support String('abc'), which is an
                // Object whose prototype is a String...
                _ => fail!("charCodeAt called on a non-string")
            };
            JsNumber(rv)
        };
        do self.add_native_func_str(frame, self.myString, "substring")
            |_this, _args| {
            fail!("String.substring() unimplemented");
        };
        do self.add_native_func_str(frame, self.myString, "valueOf")
            |this, _args| {
            match this {
                JsString(_) => this,
                JsObject(_) => fail!("wrapped string valueOf unimplemented"),
                _ =>
                fail!("TypeError: String.prototype.valueOf is not generic")
            }
        };
        do self.add_native_func_str(frame, myStringCons, "fromCharCode")
            |_this, _args| {
            fail!("String.fromCharCode() unimplemented");
        };
        do self.add_native_func_str(frame, self.myMath, "floor")
            |_this, args| {
            JsNumber(self.toNumber(getarg(args, 0)).floor())
        };
        do self.add_native_func_str(frame, self.myNumber, "toString")
            |this, args| {
            let n = match this {
                JsNumber(n) => n,
                _ => fail!("TypeError: Number.prototype.toString is not generic")
            };
            let radix = match getarg(args, 0) {
                JsUndefined => 10u,
                JsNumber(n) if n >= 2f64 && n <= 36f64 => n as uint,
                // XXX should throw
                _ => fail!("RangeError: toString() radix argument must be between 2 and 36")
            };
            let s = match n {
                f64::infinity => ~"Infinity",
                f64::neg_infinity => ~"-Infinity",
                _ if n.is_NaN() => ~"NaN",
                _ => f64::to_str_radix(n, radix)
            };
            JsVal::from_str(s)
        };
        do self.add_native_func_str(frame, self.myNumber, "valueOf")
            |this, _args| {
            match this {
                JsNumber(_) => this,
                _ => fail!("TypeError")
            }
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

        // Object.Try/Object.Throw -- turtlescript extension!
        do self.add_native_func_str(frame, myObjectCons, "Try")
            |_this, args| {
            let this = getarg(args, 0);
            let bodyBlock = getarg(args, 1);
            let catchBlock = getarg(args, 2);
            let finallyBlock = getarg(args, 3);
            let mut rv = self.interpret_function(bodyBlock, this, ~[]);
            match (rv, catchBlock) {
                (JsThrown(v), JsObject(_)) => {
                    // exception caught! invoke catchBlock!
                    //io::println("exception caught!");
                    self.interpret_function(catchBlock, this, ~[*v]);
                    rv = JsUndefined;
                },
                _ => { /* no catch block; keep throwing */ }
            };
            match finallyBlock {
                JsObject(_) => {
                    fail!("finally unimplemented");
                },
                _ => { /* no finally block */ }
            };
            rv
        };
        do self.add_native_func_str(frame, myObjectCons, "Throw")
            |_this, args| {
            JsThrown(@getarg(args, 0))
        };

        frame
    }

    fn isCallable(&self, val: JsVal) -> bool {
        match val {
            JsObject(_) => match self.get_slot_fd(val, self.fdValue) {
                JsNativeFunction(_) | JsFunctionCode(_) => true,
                _ => false
            },
            _ => false
        }
    }

    fn toObject(&self, val: JsVal) -> @mut Object {
        match val {
            JsUndefined | JsNull => fail!("TypeError"), // xxx throw
            JsObject(obj) => obj,
            // should create wrapper types for JsBool,JsNumber,JsString
            _ => fail!("unimplemented")
        }
    }

    priv fn toPrimitive(&self, val: @mut Object, hint: &str) -> JsVal {
        let funcDefaultValue = val.get(self.fdDefaultValue);
        self.interpret_function(funcDefaultValue, JsObject(val),
                                ~[JsVal::from_str(hint)])
    }
    pub fn toString(&self, val: JsVal) -> ~str {
        match val {
            JsObject(obj) => self.toString(self.toPrimitive(obj, "String")),
            _ => val.to_str()
        }
    }
    pub fn toBoolean(&self, val: JsVal) -> bool {
        match val {
            JsUndefined | JsNull => false,
            JsBool(b) => b,
            JsNumber(n) => !(n.is_NaN() || n==0f64), //+0,-0, or NaN
            JsString(utf16) => !utf16.is_empty(),
            //JsObject(obj) => match obj.get(self.fdType)...
            JsObject(_) => true,
            _ => fail!(fmt!("unimplemented case for toBoolean: %?", val))
        }
    }
    pub fn toNumber(&self, val: JsVal) -> f64 {
        // this is the conversion done by (eg) bi_mul
        match val {
            JsObject(obj) => {
                self.toNumber(self.toPrimitive(obj, "Number"))
            },
            JsString(utf16) => {
                let s = str::from_utf16(utf16);
                // XXX shouldn't have to break up this expression (rust bug)
                match s.trim() {
                    // these are rust constants, not javascript
                    "inf" | "+inf" | "-inf" => f64::NaN,
                    // these are the javascript names (which rust doesn't accept)
                    "Infinity" | "+Infinity" => f64::infinity,
                    "-Infinity" => f64::neg_infinity,
                    // empty string is zero
                    "" => 0f64,
                    // use the rust from_str method for everything else
                    // XXX should support 0xNN format.
                    s => match f64::from_str(s) {
                        Some(n) => n,
                        None => f64::NaN
                    }
                }
            },
            JsNumber(n) => n,
            JsUndefined => f64::NaN,
            JsBool(false) | JsNull => 0f64,
            JsBool(true) => 1f64,
            _ => fail!(fmt!("can't convert %? to number", val))
        }
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
            JsThrown(_) | JsFunctionCode(_) | JsNativeFunction(_) => {
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

    priv fn throw(&self, mut state: ~State, ex: @JsVal) -> ~State {
        // JsVal should be instance of JsThrown
        // XXX set private 'stack' field to the frame (assuming frame stores
        // function names)
        while state.parent.is_some() {
            // use pattern matching to work around a limitation of the
            // type system; ideally this should work:
            //state = state.parent.expect("throw from top of stack");
            let ~State { parent: parent, _ } = state;
            state = parent.expect("throw from top of stack");
        }
        state.stack.push(JsThrown(ex));
        state
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
            _f => {
                // xxx throw wrapped TypeError
                fail!(fmt!("Not a function at %u function %u: %?", state.pc, state.function.id, _f));
            }
        };
        self.invoke_internal(state, func, my_this, native_args)
    }
    priv fn invoke_internal(&self, mut state: ~State, func: @mut Object,
                            this: JsVal, args: ~[JsVal]) -> ~State {
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
                let rv = f(this, args);
                // handle "apply-like" natives
                match (func.get(self.fdIsApply), rv) {
                    (JsBool(true), _) => {
                        let mut nArgs = 0u;
                        for self.arrayEach(rv) |v| {
                            state.stack.push(v);
                            nArgs += 1;
                        }
                        return self.invoke(state, nArgs-2);
                    },
                    (_, JsThrown(ex)) => {
                        return self.throw(state, ex);
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
                }, this);
                nframe.set(FieldDesc {
                    name: intern("arguments"), hidden: false
                }, self.arrayCreate(args));
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

    // interpret a function object stored in a JsVal
    pub fn interpret_function(&self, function: JsVal,
                              this: JsVal, args: ~[JsVal]) -> JsVal {
        // lookup the module and function id from the function JsVal
        match (self.get_slot_fd(function, self.fdValue),
               self.get_slot_fd(function, self.fdParentFrame)) {
            (JsNativeFunction(f), _) => {
                let rv = f(this, args);
                // "apply-like" natives
                match self.get_slot_fd(function, self.fdIsApply) {
                    JsBool(true) => {
                        let mut nargs : ~[JsVal] = ~[];
                        for self.arrayEach(rv) |v| {
                            nargs.push(v);
                        }
                        let nfunction = nargs.shift();
                        let nthis = nargs.shift();
                        self.interpret_function(nfunction, nthis, nargs)
                    },
                    _ => rv // might be a throw exception
                }
            },
            (JsFunctionCode(f), JsObject(parent_frame)) => {
                // make a frame for the function invocation
                let nframe = Object::create(self.root_map, parent_frame);
                nframe.set(FieldDesc {
                    name: intern("this"), hidden: false
                }, this);
                nframe.set(FieldDesc {
                    name: intern("arguments"), hidden: false
                }, self.arrayCreate(args));
                self.interpret(f.module, f.function.id, Some(nframe))
            },
            _ => fail!("not a function")
        }
    }

    // interpret a function (typically the module initializer)
    pub fn interpret(&self, module: @Module, func_id: uint, frame: Option<@mut Object>) -> JsVal {
        let frame2 = match frame {
            Some(f) => f,
            None => self.make_top_level_frame(JsNull, ~[])
        };
        let function = module.functions[func_id];
        let top = ~State::new(None, frame2, module, function);
        let mut state = ~State::new(Some(top), frame2, module, function);
        while state.parent.is_some() /* wait for state == top */ {
            state = self.interpret_one(state);
        }
        state.stack.pop()
    }

    // take one step in the interpreter (ie interpret one bytecode op)
    pub fn interpret_one(&self, mut state: ~State) -> ~State {
        //io::println(fmt!("fid %u pc %u stack %?", state.function.id, state.pc, state.stack.len()));
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
                        JsBool(b),
                        //JsObject(if b { self.myTrue } else { self.myFalse }),
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
                if !self.toBoolean(cond) {
                    state.pc = arg1;
                }
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
                JsBool(!self.toBoolean(arg))
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
                let rv = match (left, right) {
                    (JsNumber(l), JsNumber(r)) => (l == r),
                    (JsString(l), JsString(r)) => (l == r),
                    (JsBool(l), JsBool(r)) => (l == r),
                    (JsObject(l), JsObject(r)) => ptr::ref_eq(l, r),
                    (JsNull, JsNull) | (JsUndefined, JsUndefined) => true,
                    (JsObject(_), _) |
                    (JsNumber(_), _) |
                    (JsBool(_),   _) |
                    (JsString(_), _) |
                    (JsNull,      _) |
                    (JsUndefined, _)  => false,
                    _ => fail!(fmt!("func %u pc %u: unimplemented case for bi_eq: %s %s", state.function.id, state.pc, left.to_str(), right.to_str()))
                };
                JsBool(rv)
            },
            Op_bi_gt => do self.binary(state) |left, right| {
                let rv = match (left, right) {
                    (JsString(l), JsString(r)) => (l > r),
                    (_, JsNumber(_)) |
                    (JsNumber(_), _) => (self.toNumber(left) > self.toNumber(right)),
                    _ => fail!(fmt!("func %u pc %u: unimplemented case for bi_gt: %s %s", state.function.id, state.pc, left.to_str(), right.to_str()))
                };
                JsBool(rv)
            },
            Op_bi_gte => do self.binary(state) |left, right| {
                let rv = match (left, right) {
                    (JsString(l), JsString(r)) => (l >= r),
                    (_, JsNumber(_)) |
                    (JsNumber(_), _) => (self.toNumber(left) >= self.toNumber(right)),
                    _ => fail!(fmt!("func %u pc %u: unimplemented case for bi_gte: %s %s", state.function.id, state.pc, left.to_str(), right.to_str()))
                };
                JsBool(rv)
            },
            Op_bi_add => do self.binary(state) |left, right| {
                let lprim = match left {
                    JsObject(obj) => self.toPrimitive(obj, ""),
                    _ => left
                };
                let rprim = match right {
                    JsObject(obj) => self.toPrimitive(obj, ""),
                    _ => right
                };
                match (lprim,rprim) {
                    // XXX we really need a faster algorithm for
                    // string concat
                    (JsString(l), JsString(r)) => JsString(l + r),
                    (JsString(_),_) | (_,JsString(_)) => {
                        // XXX even slower!
                        JsVal::from_str(self.toString(lprim) +
                                        self.toString(rprim))
                    },
                    _ => JsNumber(self.toNumber(lprim) + self.toNumber(rprim))
                }
            },
            Op_bi_sub => do self.binary(state) |left, right| {
                JsNumber(self.toNumber(left) - self.toNumber(right))
            },
            Op_bi_mul => do self.binary(state) |left, right| {
                JsNumber(self.toNumber(left) * self.toNumber(right))
            },
            Op_bi_div => do self.binary(state) |left, right| {
                JsNumber(self.toNumber(left) / self.toNumber(right))
            }
        }
        state
    }
}

struct Interpreter {
    pub env: ~Environment,
    priv frame: @mut Object,
    priv compile_from_source: JsVal,
    priv repl: JsVal
}
impl Interpreter {
    pub fn new() -> Interpreter {
        // create an environment and run the startup code
        let env = Environment::new();
        let module = @Module::new_startup_module();
        let frame = env.make_top_level_frame(JsNull, ~[]);
        let compile_from_source = env.interpret(module, 0, Some(frame));
        // create repl
        let make_repl = env.get_slot(compile_from_source,
                                     JsVal::from_str(~"make_repl"));
        let repl = env.interpret_function(make_repl, JsNull, ~[]);
        Interpreter {
            env: env,
            frame: frame,
            compile_from_source: compile_from_source,
            repl: repl
        }
    }
    pub fn interpret(&self, source: &str) -> JsVal {
        // compile source to bytecode
        let bc = self.env.interpret_function(
            self.compile_from_source, JsNull,
            ~[JsVal::from_str(source)]);
        // create a new module from the bytecode
        let mut buf : ~[u8] = ~[];
        for self.env.arrayEach(bc) |val| {
            buf.push(self.env.toNumber(val) as u8);
        }
        let nm = @Module::new_from_bytes(buf);
        //io::println(fmt!("module: %?", nm));
        // execute the new module.
        self.env.interpret(nm, 0, Some(self.frame))
    }
    pub fn repl(&self, source: &str) -> JsVal {
        // compile source to bytecode
        let bc = self.env.interpret_function(
            self.repl, JsNull,
            ~[JsVal::from_str(source)]);
        match bc {
            JsThrown(_) => { return bc; }, // parser exception
            _ => {}
        };
        // create a new module from the bytecode
        let mut buf : ~[u8] = ~[];
        for self.env.arrayEach(bc) |val| {
            buf.push(self.env.toNumber(val) as u8);
        }
        let nm = @Module::new_from_bytes(buf);
        // execute the new module.
        self.env.interpret(nm, 0, Some(self.frame))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_interpret1() {
        let source = ~"{ return 42; }";
        let i = Interpreter::new();
        let rv = i.interpret(source);
        assert_eq!(rv.to_str(), ~"42")
    }

    #[test]
    fn test_interpret2() {
        let source = ~"{ var x = 42; return x; }";
        let i = Interpreter::new();
        let rv = i.interpret(source);
        assert_eq!(rv.to_str(), ~"42")
    }

    fn script_test(script: &[(~str,~str)]) {
        let i = Interpreter::new();
        for script.each() |&(given, expected)| {
            let rv = i.repl(given);
            //io::println(fmt!("%s -> %s (expected %s)", given,
            //                 rv.to_str(), expected));
            assert_eq!(rv.to_str(), expected);
        }
    }

    #[test]
    fn test_repl1() {
        script_test(~[
            (~"1 + 2", ~"3"),
            (~"var x = 4*10 + 2;", ~"undefined"),
            (~"x", ~"42"),
            (~"console.log('seems to work');", ~"undefined"),
            (~"var fib = function(n) { return (n<2) ? 1 : fib(n-1) + fib(n-2); };", ~"undefined"),
            (~"fib(10)", ~"89")
        ]);
    }

    #[test]
    fn test_parseInt() {
        script_test(~[
            // sanity check numeric types
            (~"NaN", ~"NaN"),
            (~"Infinity", ~"Infinity"),
            (~"-Infinity", ~"-Infinity"),
            // test parseInt
            (~"parseInt('10', 16)", ~"16"),
            (~"parseInt('10', '16')", ~"16"),
            (~"parseInt('10', -10)", ~"NaN"),
            (~"parseInt('10', -1)", ~"NaN"),
            (~"parseInt('10', 'a')", ~"10"),
            (~"parseInt('10', 'ab')", ~"10"),
            (~"parseInt('10', NaN)", ~"10"),
            (~"parseInt('10', 'NaN')", ~"10"),
            (~"parseInt('10', Infinity)", ~"10"),
            (~"parseInt('10', 'Infinity')", ~"10"),
            (~"parseInt('10', -Infinity)", ~"10"),
            (~"parseInt('10', '-Infinity')", ~"10"),
            (~"parseInt('11')", ~"11"),
            //(~"parseInt('11z')", ~"11"), // xxx currently fails
            //(~"parseInt(' 11z')", ~"11"), // xxx currently fails
            (~"parseInt('10', '16.5')", ~"16"),
            (~"parseInt('10', 16.5)", ~"16"),
        ]);
    }

    #[test]
    fn test_cmp() {
        script_test(~[
            (~"'2' > '10'", ~"true"),
            (~"2 > 10", ~"false"),
            (~"2 > '10'", ~"false"),
            (~"'2' > 10", ~"false"),
            (~"'2' >= '10'", ~"true"),
            (~"2 >= 10", ~"false"),
            (~"2 >= '10'", ~"false"),
            (~"'2' >= 10", ~"false"),
            (~"'z' > 10", ~"false"),
            (~"'z' < 10", ~"false")
        ]);
    }

    #[test]
    fn test_mul() {
        script_test(~[
            (~"' 10z' * 1", ~"NaN"),
            (~"' 10 ' * 1", ~"10")
        ]);
    }

    #[test]
    fn test_Number_toString() {
        script_test(~[
            (~"Infinity.toString()", ~"Infinity"),
            (~"Infinity.toString(16)", ~"Infinity"),
            (~"NaN.toString(16)", ~"NaN")
        ]);
    }

    #[test]
    fn test_String_charAt() {
        script_test(~[
            (~"'abc'.charAt()", ~"a"),
            (~"'abc'.charAt(-1)", ~""),
            (~"'abc'.charAt(1)", ~"b"),
            (~"'abc'.charAt(4)", ~""),
            (~"'abc'.charAt(NaN)", ~"a"),
            (~"'abc'.charAt('a')", ~"a"),
            (~"'abc'.charAt(1.2)", ~"b"),
            (~"'abc'.charAt(2.9)", ~"c"),
        ]);
    }

    #[test]
    fn test_Math_floor() {
        script_test(~[
            (~"Math.floor(-1.1)", ~"-2"),
            (~"Math.floor(-1)", ~"-1"),
            (~"Math.floor(0)", ~"0"),
            (~"Math.floor(3)", ~"3"),
            (~"Math.floor(3.2)", ~"3"),
            (~"Math.floor({})", ~"NaN"),
            (~"Math.floor([])", ~"0"),
            (~"Math.floor([1])", ~"1"),
            (~"Math.floor([1,2])", ~"NaN"),
            (~"Math.floor('abc')", ~"NaN"),
            (~"Math.floor(' 10 ')", ~"10"),
            (~"Math.floor()", ~"NaN"),
        ]);
    }

    #[test]
    fn test_Boolean() {
        script_test(~[
            (~"Boolean(true)", ~"true"),
            (~"Boolean(false)", ~"false"),
            (~"Boolean(0)", ~"false"),
            (~"Boolean(NaN)", ~"false"),
            (~"Boolean('abc')", ~"true"),
            (~"Boolean('')", ~"false"),
            (~"Boolean(123)", ~"true"),
        ]);
    }

    #[test]
    fn test_toNumber() {
        script_test(~[
            (~"11 * 1", ~"11"),
            (~"' 11\\n' * 1", ~"11"),
            (~"' -11\\n' * 1", ~"-11"),
            (~"true * 1", ~"1"),
            (~"false * 1", ~"0"),
            (~"null * 1", ~"0"),
            (~"undefined * 1", ~"NaN"),
            (~"'xxx' * 1", ~"NaN"),
            (~"'Infinity' * 1", ~"Infinity"),
            (~"'-Infinity' * 1", ~"-Infinity"),
            (~"'inf' * 1", ~"NaN"),
            (~"'-inf' * 1", ~"NaN"),
            (~"'NaN' * 1", ~"NaN"),
            (~"1e1 * 1", ~"10"),
            (~"'1e1' * 1", ~"10"),
            //(~"'0x10' * 1", ~"16"),// not yet supported
            (~"'' * 1", ~"0"),
        ]);
    }

    #[test]
    fn test_obj_eq() {
        script_test(~[
            (~"var x = {};", ~"undefined"),
            (~"var y = { f: x };", ~"undefined"),
            (~"var z = { f: x };", ~"undefined"),
            (~"y===z", ~"false"),
            (~"x===x", ~"true"),
            (~"y.f === z.f", ~"true"),
            (~"z.f = {};", ~"undefined"),
            (~"y.f === z.f", ~"false"),
        ]);
    }

    #[test]
    fn test_String_valueOf() {
        script_test(~[
            (~"var x = 'abc';", ~"undefined"),
            (~"x.valueOf()", ~"abc"),
            (~"x.toString()", ~"abc"),
            (~"x === x.valueOf()", ~"true"),
            (~"x === x.toString()", ~"true"),
            (~"x === x", ~"true"),
            // XXX: now with a wrapped string object
        ]);
    }

    #[test]
    fn test_Array_join() {
        script_test(~[
            (~"var a = [1,2,3];", ~"undefined"),
            (~"a.toString()", ~"1,2,3"),
            (~"a.join(':')", ~"1:2:3"),
            (~"a.join(4)", ~"14243"),
        ]);
    }
}
