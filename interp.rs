use op::*;
use function::Function;
use module::Module;
use object::*;
use intern::intern;

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
    myMath: @mut Object
}

impl Environment {
    pub fn new() -> ~Environment {
        let root_map = @mut ObjectMap::new();
        let fdType = FieldDesc { name: intern("type"), hidden: true };
        let fdValue = FieldDesc { name: intern("value"), hidden: true };

        let myObject = Object::new(root_map); // parent of all objects.
        //myObject.get(fdType);
        myObject.set(fdType, JsVal::from_str("object"));

        let myArray = Object::create(root_map, myObject);
        myArray.set(fdType, JsVal::from_str("array"));
        myArray.set(FieldDesc { name: intern("length"), hidden: false },
                    JsNumber(0f64));

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
            myMath: myMath
        }
    }

    fn add_native_func(&self, frame : @mut Object,
                       obj : @mut Object, desc: FieldDesc,
                       f : NativeFunction) -> @mut Object {
        let my_func = Object::create(self.root_map, self.myFunction);
        my_func.set(FieldDesc { name: intern("parent_frame"), hidden: true },
                    JsObject(frame));
        my_func.set(FieldDesc { name: intern("value"), hidden: true },
                    JsNativeFunction(f));
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
        myArgs.set(FieldDesc { name: intern("length"), hidden: false },
                   JsNumber(arguments.len() as f64));
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

    pub fn interpret(&self, mut state: ~State) -> ~State {
        let op = Op::new_from_uint(state.function.bytecode[state.pc]);
        state.pc += 1;
        let arg1;
        match op.args() {
            0 => { arg1 = 0; }
            1 => { arg1 = state.function.bytecode[state.pc]; state.pc +=1; }
            _ => fail!()
        }
        let mut ns = state;
        match op {
            Op_push_frame => {
            },
            Op_push_literal => {
            },
            Op_get_slot_direct => {
            },
            Op_get_slot_direct_check => {
            },
            Op_invoke => {
            },
            Op_return => {
            },
            Op_pop => {
            },
            Op_dup => {
            },
            Op_swap => {
            },
            _ => fail!() // unimplemented
        }
        ns
    }
}
