use op::Op;
use function::Function;
use literal::Literal;
use module::Module;
use object::*;
use intern::intern;

struct State {
    // main interpreter state.
    parent: Option<State>, // calling context (another state)
    frame: ~Object,
    stack: ~[JsVal],
    pc: uint,
    // from bytecode file
    module: Module,
    func_id: uint,
    // cached
    bytecode: ~[uint],
    literals: ~[Literal]
}

// helper function
fn mkJsString(s: &str) -> JsVal {
    JsString(at_vec::to_managed_consume(str::to_utf16(s)))
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
        myObject.set(fdType, mkJsString("object"));

        let myArray = Object::create(root_map, myObject);
        myArray.set(fdType, mkJsString("array"));
        myArray.set(FieldDesc { name: intern("length"), hidden: false },
                    JsNumber(0f64));

        let myFunction = Object::create(root_map, myObject);
        myFunction.set(fdType, mkJsString("function"));
        myFunction.set(fdValue, JsUndefined); // allocate space

        let myString = Object::create(root_map, myObject);
        myString.set(fdType, mkJsString("string"));
        //myString.set(fdValue, JsUndefined); // allocate space

        let myNumber = Object::create(root_map, myObject);
        myNumber.set(fdType, mkJsString("number"));

        let myBoolean = Object::create(root_map, myObject);
        myBoolean.set(fdType, mkJsString("boolean"));

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
        frame.set(FieldDesc { name: intern("console"), hidden: false },
                  JsObject(Object::create(self.root_map, self.myObject)));

        // XXX hook up native functions

        frame
    }
}
