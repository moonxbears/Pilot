using System;
using System.Collections.Generic;
using System.IO;
using Antlr4.Runtime;
using Xunit;

namespace Pilot.Tests
{
    public static class ParserHelper
    {
        public static (PilotParser.ProgramContext Tree, List<string> Errors) Parse(string code)
        {
            var inputStream = new AntlrInputStream(code);
            var lexer = new PilotLexer(inputStream);
            var tokenStream = new CommonTokenStream(lexer);
            var parser = new PilotParser(tokenStream);

            var errorTracker = new ErrorTracker();
            parser.RemoveErrorListeners();
            parser.AddErrorListener(errorTracker);

            var tree = parser.program();
            return (tree, errorTracker.Errors);
        }
    }

    public class ErrorTracker : BaseErrorListener
    {
        public List<string> Errors { get; } = new List<string>();

        public override void SyntaxError(
            IRecognizer recognizer,
            IToken offendingSymbol,
            int line,
            int charPositionInLine,
            string msg,
            RecognitionException e)
        {
            Errors.Add($"[{line}:{charPositionInLine}] {msg} (at '{offendingSymbol?.Text}')");
        }
    }

    public class TypeSafetyTests
    {
        [Fact]
        public void Test_Primitive_And_Compound_Variable_Declarations()
        {
            string code = @"
let a: int = 10
var b: float = 3.14
const c: bool = True
let user_opt: User? = null
let pair: (int, str) = (1, ""Pilot"")
let union_val: int | str = 42
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Struct_Class_Interface_Enum_Definitions()
        {
            string code = @"
struct Vector3 {
    let x: float
    let y: float
    let z: float
}

class Account {
    def __init__(self, owner: str) -> void {
        self.owner = owner
    }
}

interface Reader[T] {
    def read() -> Result[T, str]
}

enum Status {
    Idle,
    Active(float),
    Failed(int)
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Pattern_Matching()
        {
            string code = @"
def evaluate(s: Status) -> str {
    match s {
        case Status.Active(rate) if rate > 10.0 {
            return ""High activity""
        }
        case Status.Idle {
            return ""Sleeping""
        }
        case _ {
            return ""Default""
        }
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }
    }

    public class DatabaseSetOperationsTests
    {
        [Fact]
        public void Test_Table_Declaration_With_Indexes()
        {
            string code = @"
var users: Table[User] index by (id [hash, unique], email [hash], age [btree])
var txns: Table[Txn] index by (id [hash], user_id [btree])
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_LINQ_Style_Query_With_Joins_Where_And_Aggregation()
        {
            string code = @"
def get_vips() -> List[Summary]:
    let report = (from u in users with index(age)
                  join t in txns on u.id == t.user_id
                  where u.age >= 21 and t.amount > 100.0
                  group t by u.name into g having sum(g.amount) > 1000.0
                  order by sum(g.amount) descending
                  select { customer: g.key, total: sum(g.amount) }
                  limit 50 offset 10)
    return report
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_SQL_First_Query_And_Set_Operations()
        {
            string code = @"
def run_relational_ops() -> Dataset[User]:
    let active = (select u.name, u.email from users as u where u.is_active == True)
    let premium = (select u.name, u.email from users as u where u.tier == ""gold"")

    let union_all_res = active union all premium
    let intersect_res = active intersect premium
    let except_res = active except premium
    let point_lookup = lookup users with index(email) where u.email == ""alice@example.org""
    return point_lookup
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }
    }

    public class ConcurrencyTests
    {
        [Fact]
        public void Test_Async_Await_And_Actor()
        {
            string code = @"
actor TelemetryHub:
    var packets: int = 0
    async def log_packet(self, data: str) -> void:
        self.packets += 1

async def fetch_remote(client: Client) -> str:
    let result = await client.request_async()
    return result
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Channels_And_CSP_Select()
        {
            string code = @"
async def channel_handler(in_ch: Channel[int], out_ch: Channel[str]) -> void {
    select {
        case msg = <-in_ch {
            out_ch <- f""Got {msg}""
        }
        case out_ch <- ""ping"" {
            pass
        }
        timeout 50ms {
            log(""Timeout"")
        }
        default {
            pass
        }
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Spawn_Parallel_For_And_Atomic_Sync_Isolate()
        {
            string code = @"
def execute_multithreaded(items: List[int]) -> void {
    spawn {
        worker()
    }

    parallel for item in items {
        process(item)
    }

    atomic {
        counter += 1
    }

    sync(mutex) {
        shared_state.update()
    }

    isolate {
        run_isolated()
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }
    }

    public class RealTimeTests
    {
        [Fact]
        public void Test_Hard_RealTime_With_NoAlloc_StackAlloc_And_Bounded_Loops()
        {
            string code = @"
@hard_realtime
@wcet(250us)
def flight_control(cmd: int) -> void {
    noalloc {
        var buf = stackalloc float[64]
        for i in 0..64 bounded 64 {
            buf[i] = compute_step(i)
        }
        
        while not check_ready() bounded 10 {
            pass
        }
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Soft_RealTime_Deadlines_Budgets_And_Periodic_Tasks()
        {
            string code = @"
@soft_realtime
def control_routine() -> void {
    deadline 10ms jitter 500us {
        calculate_next_state()
    } on_missed {
        safe_stop()
    }

    within 5ms {
        render_frame()
    } fallback {
        reuse_last_frame()
    }

    periodic 20ms priority 99 jitter 50us {
        poll_can_bus()
    }

    region dsp_arena {
        process_samples()
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Temporal_Frequency_And_Memory_Literals()
        {
            string code = @"
let t1 = 10ns
let t2 = 250us
let t3 = 50ms
let t4 = 2s
let freq1 = 50Hz
let freq2 = 10kHz
let freq3 = 2.4GHz
let mem1 = 64KB
let mem2 = 4MB
let mem3 = 16GB
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }
    }

    public class UdsaMetaprogrammingTests
    {
        [Fact]
        public void Test_Units_Of_Measure_And_Dimensional_Types()
        {
            string code = @"
unit meter
unit second
unit speed = meter / second

let dist: float<meter> = 100.0<meter>
let t: float<second> = 9.58<second>
let v: float<meter / second> = dist / t
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Embedded_DSL_And_Pipeline_Operators()
        {
            string code = @"
dsl neural_network(layers: int) {
    dense(128, activation=""relu"")
    dropout(0.2)
    dense(10, activation=""softmax"")
}

def transform(items: List[int]) -> List[int] {
    let result = items |> filter(is_valid) |> map(scale)
    let back = format <| result
    return result
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Compile_Time_Macros_And_Quotes()
        {
            string code = @"
macro make_getter(field) {
    quote {
        def get_val(self) -> any {
            return getattr(self, $field)
        }
    }
}

def get_ast() -> AstNode {
    let tree = quote { 10 + 20 * 30 }
    return tree
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }
    }

    public class GuardDeferTests
    {
        [Fact]
        public void Test_Guard_Statements_And_Preconditions()
        {
            string code = @"
def validate_age(age: int) -> bool {
    guard age >= 18 else {
        return false
    }

    guard age < 120 else {
        raise ValueError(""Unrealistic age"")
    }

    return true
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Guard_Let_Pattern_Preconditions()
        {
            string code = @"
def handle_request(req_id: int) -> str {
    guard let req = fetch_request(req_id) else {
        return ""not_found""
    }

    guard req.is_valid else {
        raise InvalidRequestError(""Malformed"")
    }

    return req.payload
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Defer_Single_And_Block_Cleanup()
        {
            string code = @"
def execute_pipeline() {
    conn = open_connection()
    defer conn.close()

    defer {
        flush_buffers()
        release_locks()
    }

    var count: int = 0
    defer count += 1
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }
    }

    public class PostgresDatabaseTests
    {
        [Fact]
        public void Test_Generic_Database_Connections()
        {
            string code = @"
database pg = connect(""postgres://localhost:5432/db"")
database sqlite = connect(""sqlite:///local.db"")
database memory = connect(""memory://"")
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Postgres_Insert_On_Conflict_And_Returning()
        {
            string code = @"
def create_customer(c: Customer) -> int {
    let new_id = insert into db.customers (name, email)
        values (c.name, c.email)
        on conflict (email) do update set name = c.name
        returning id
    return new_id
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Postgres_Update_And_Delete_With_Returning()
        {
            string code = @"
def update_and_purge() {
    let updated = update db.users
        set active = false
        where last_login < cutoff
        returning id, email

    let purged = delete from db.logs
        where created_at < cutoff
        returning count()
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Window_Functions_Over_Partition_Order()
        {
            string code = @"
def rank_salaries() {
    let ranked = select
            e.name,
            rank() over (partition by e.dept_id order by e.salary desc) as rnk,
            sum(e.salary) over (partition by e.dept_id) as total_dept
        from db.employees as e
    return ranked
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Jsonb_Operators()
        {
            string code = @"
def search_jsonb() {
    let matches = select u.id
        from db.users as u
        where u.meta ->> ""role"" == ""admin""
          and u.permissions @> [""read"", ""write""]
    return matches
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Transactions_Savepoints_And_Rollback()
        {
            string code = @"
def process_tx() {
    transaction(db) {
        update db.accounts set bal = bal - 100 where id == 1
        savepoint debited
        rollback to savepoint debited
        commit transaction
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }
    }

    public class CSharpRobustnessTests
    {
        [Fact]
        public void Test_Properties_With_Get_And_Set()
        {
            string code = @"
class Wallet {
    var _funds: float = 0.0

    property funds: float {
        get {
            return self._funds
        }
        set(v) {
            guard v >= 0.0 else {
                raise ValueError(""Funds must be >= 0"")
            }
            self._funds = v
        }
    }

    property id: str { get; set; }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Operator_Overloading()
        {
            string code = @"
struct Complex {
    let real: float
    let imag: float

    operator + (self, other: Complex) -> Complex {
        return Complex(self.real + other.real, self.imag + other.imag)
    }

    operator == (self, other: Complex) -> bool {
        return self.real == other.real and self.imag == other.imag
    }
}

class IntList {
    var items: list[int] = []

    operator [] (self, idx: int) -> int {
        return self.items[idx]
    }

    operator []= (self, idx: int, val: int) -> void {
        self.items[idx] = val
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Extension_Methods()
        {
            string code = @"
extension StringOps for str {
    def is_empty(self) -> bool {
        return len(self) == 0
    }
}

extension for list[T] where T: Printable {
    def display_all(self) -> void {
        for x in self {
            print(x)
        }
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Generic_Type_Constraints()
        {
            string code = @"
def inspect[T](val: T) -> str where T: Printable & Serializable {
    return val.serialize()
}

class Store[T, K] where T: Identifiable[K], K: Hashable {
    def lookup(self, key: K) -> T? {
        return null
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Null_Coalescing_And_Safe_Navigation()
        {
            string code = @"
def process(user: User?, opts: Config?) {
    var config = opts
    config ??= Config()

    let name = user?.profile?.name ?? ""Guest""
    return name
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }
    }

    public class ExtendedLanguageFeaturesTests
    {
        [Fact]
        public void Test_Accessibility_Modifiers()
        {
            string code = @"
public def public_func() -> int { return 1; }
private def private_func() -> void { }
protected def protected_func() -> void { }
internal def internal_func() -> void { }

public class PublicClass {
    public def method() -> void { }
    private var secret: int = 42
}

protected class ProtectedClass { }
internal struct InternalPoint {
    public let x: int
    public let y: int
}

public interface IPublicService {
    def execute() -> void
}

public enum Status {
    Active,
    Inactive
}

public let global_name: str = ""Pilot""
private var internal_counter: int = 0
public const MAX_CONNECTIONS: int = 1000
public macro public_macro(msg) { print(msg); }
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Macros_Definitions_And_Invocations()
        {
            string code = @"
macro trace(msg) {
    print(msg);
}

macro! compile_time_eval() -> str {
    return ""evaluated"";
}

def run() -> void {
    println!(""Hello, {}!"", ""Pilot"");
    let numbers = vec![1, 2, 3, 4, 5];
    let query_res = sql! {
        select id, name from users where active = true;
    };
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Destructuring_Unpacking_Declarations_And_Loops()
        {
            string code = @"
def test_unpacking() -> void {
    let (a, b) = (1, 2)
    let (x: int, y: int) = (10, 20)
    let [first, *rest] = [1, 2, 3, 4]
    let {id, name} = get_user()
    let {id: uid, name: uname} = get_user()
    var (w, h) = (800, 600)
    const (MIN, MAX) = (0, 100)
    let ((x1, y1), [z1, *remaining]) = complex_data

    for (k, v) in pairs {
        print(k)
    }

    for [x, y] in coordinates {
        print(x + y)
    }

    for {id, name} in user_list {
        print(id)
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Keywords_Symbol_BreakLabel_Ref_Dynamic()
        {
            string code = @"
def test_keywords() -> void {
    // symbol keyword
    let s1: symbol = :active
    let s2: symbol = symbol(""pending"")
    let s3 = :error_code

    // break and continue to label
    outer: for i in 0..10 {
        inner: while True {
            if i > 5 {
                break outer;
            } else {
                continue outer;
            }
        }
    }

    named_block: {
        let x = 10
        if x > 5 {
            break named_block;
        }
    }

    // ref keyword
    var counter = 0
    ref r = counter
    let f = [ref counter]() => { counter += 1; }

    // dynamic keyword
    dynamic dyn_obj = get_external_service()
    let dyn_val: dynamic = dyn_obj.invoke(""test"")
}

def swap(ref a: int, ref b: int) -> void {
    let tmp = a
    a = b
    b = tmp
}

def test_swap_call() -> void {
    var x = 1
    var y = 2
    swap(ref x, ref y)
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Modern_Lambda_And_Closure_Syntax()
        {
            string code = @"
def test_lambdas() -> void {
    let factor = 10
    // [capture](args) => expression
    let mult = [factor](x: int) => x * factor;

    // [capture](args) => { code block }
    let logger = [](msg: str) => {
        print(msg);
        return True;
    };

    // [capture](args) -> return_type { code block }
    var total = 0
    let accum = [ref total](item: int) -> int {
        total += item;
        return total;
    };

    // (args) => expression
    let add = (a: int, b: int) => a + b;

    // x => expression
    let double_val = x => x * 2;

    // x => { code block }
    let complex_mapper = x => {
        let squared = x * x;
        return squared + 1;
    };

    // def anonymous function
    let anon1 = def(x: int) -> int { return x * 3; };
    let anon2 = fn(x) => x + 10;

    // Pipeline with modern closure
    let items = [1, 2, 3] |> map((x) => x * 2) |> filter(x => x > 2);

    // Legacy lambda syntax backwards compatibility
    let legacy = lambda x, y: x + y;
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Cascade_Operator_And_Chaining_With_Interruption()
        {
            string code = @"
class Builder {
    def myfunc1(self) -> void { }
    def myfunc2(self, a: int, b: int) -> int { return a + b; }
    def myfunc3(self, a: int, b: int) -> void { }
    def myfunc4(self) -> str { return ""done""; }
    def myfunc5(self, msg: str) -> void { }
}

def test_cascade() -> void {
    let myclass = Builder()

    // Chaining example from prompt
    myclass..myfunc1()
    ..myfunc2(arg1, arg2)
    ..myfunc3(arg1, kwarg1)

    // Partial interruption to assign method return value (with let, arrow, or direct assignment)
    myclass..myfunc1()
    ..let res1 = myfunc2(1, 2)
    ..myfunc3(1, res1)
    ..myfunc4() -> res2
    ..myfunc5(res2)
    ..res3 = myfunc2(3, 4)
    ..myfunc3(3, res3)

    // Null-safe cascade
    let opt_builder: Builder? = null
    opt_builder?..myfunc1()..myfunc2(10, 20)

    // Range preservation check (ranges remain distinct from cascades)
    for i in 0..10 { }
    let start_idx = 0
    let end_idx = 5
    for i in start_idx..end_idx { }
    let sub = [1, 2, 3][0..2]
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Keywords_Params_Args_Kwargs_Nameof_Static()
        {
            string code = @"
public static class MathUtils {
    public static const PI: float = 3.14159
    public static var call_count: int = 0

    public static def calculate(params numbers: List[float]) -> float {
        return 0.0
    }

    public static def variadic_call(*args, **kwargs) -> void {
        pass
    }
}

def test_features() -> void {
    let x = 42
    let var_name = nameof(x)
    let method_name = nameof(MathUtils.calculate)

    MathUtils.call_count += 1
    let result = MathUtils.calculate(1.0, 2.0, 3.0)
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }
    }

    public class PatternMatchingAndCollectionsTests
    {
        [Fact]
        public void Test_CSharp_Collection_Expressions_With_Spreads()
        {
            string code = @"
def test_collections() {
    let a = [1, 2, 3]
    let b = [4, 5, 6]
    let c = [..a, ..b]
    let d = [0, ..a, 99, ..b, 100]
    let empty = []
    let dict = {..d1, ""key"": 123}
    let set_val = {..s1, 10}
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Relational_And_Combinator_Patterns()
        {
            string code = @"
def classify(x: int) -> str {
    match x {
        case > 0 and <= 100 => ""in range""
        case < 0 or > 1000 => ""out of bounds""
        case not 42 => ""not answer""
        case not null => ""valid""
        case _ => ""other""
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Slice_And_Collection_Patterns()
        {
            string code = @"
def inspect_list(items: List[int]) -> str {
    match items {
        case [] => ""empty""
        case [var first, ..var rest] when first == 0 => ""zero start""
        case [first, .., last] => ""endpoints""
        case [_, > 10, ..] => ""second > 10""
        case [..] => ""anything""
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Property_Patterns_And_Guards()
        {
            string code = @"
def test_props(obj: any) -> str {
    match obj {
        case { Name: ""Pilot"", Version: >= 2.0 } => ""modern""
        case { Count: 0 } => ""empty""
        case (var x, var y) when x == y => ""equal pair""
        case (var x, var y) if x > y => ""descending pair""
        case Point { X: 1, Y: 2 } => ""unit point""
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Is_Pattern_Expressions()
        {
            string code = @"
def test_is(val: any, items: List[int]) -> void {
    if val is > 0 and <= 100 {
        pass
    }
    if val is not null {
        pass
    }
    if items is [var first, ..var rest] {
        pass
    }
    if items is [] {
        pass
    }
    if items is [_, > 10, ..] {
        pass
    }
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }
    }

    public class BuiltInTypesAndMatrixTests
    {
        [Fact]
        public void Test_All_Numeric_And_Float_Bit_Types()
        {
            string code = @"
def test_primitives() -> void {
    let c: char = 'A'
    let s: str = ""pilot""
    let s_alias: string = ""language""

    let a8: i8 = 10
    let a16: i16 = 100
    let a32: i32 = 1000
    let a_int: int = 10000
    let a64: i64 = 100000

    let u8_val: u8 = 255
    let u16_val: u16 = 65535
    let u32_val: u32 = 4294967295
    let u_int: uint = 500
    let u64_val: u64 = 18446744073709551615

    let f8_val: f8 = 1.0
    let f16_val: f16 = 2.0
    let f32_val: f32 = 3.14
    let f_float: float = 2.718
    let f64_val: f64 = 1.414
    let d_double: double = 0.577
    let d_decimal: decimal = 99.99
    let c_complex: complex = 1.0
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Collections_Buffer_And_Nullability_Types()
        {
            string code = @"
def test_collections_and_buffers() -> void {
    let t: tuple = (1, 2)
    let l: list = [1, 2]
    let r: range = range(0, 10)
    let m: map = {""a"": 1}
    let s: set = {1, 2}
    let fs: frozenset = frozenset([1, 2])
    let b: bool = True
    let by: bytes = bytes([1, 2])
    let ba: bytearray = bytearray([1, 2])
    let mv: memoryview = memoryview(by)

    let nl: null = null
    let no: none = none
    let un: undefined = undefined
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Matrix_NxM_With_Type_Parameter()
        {
            string code = @"
def transform(m3: matrix3x3<u8>, m4: matrix4x4<f32>, m2x4: matrix2x4<f64>) -> matrix3x3<u8> {
    let view: matrix1x3<int> = mat1x3()
    return m3
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Angle_Bracket_Generics()
        {
            string code = @"
def test_generics() -> void {
    let l: list<int> = [1, 2, 3]
    let m: map<str, i32> = {""a"": 1}
    let s: set<char> = {'a', 'b'}
    let t: tuple<int, str> = (42, ""val"")
}
";
            var (tree, errors) = ParserHelper.Parse(code);
            Assert.Empty(errors);
            Assert.NotNull(tree);
        }
    }

    public class FileSuiteTests
    {
        public static IEnumerable<object[]> GetPositiveTestFiles()
        {
            string testDir = Path.Combine(AppDomain.CurrentDomain.BaseDirectory, "..", "..", "..", "..", "tests");
            if (!Directory.Exists(testDir))
            {
                testDir = Path.Combine(Directory.GetCurrentDirectory(), "tests");
            }

            foreach (var file in Directory.GetFiles(testDir, "*.pilot"))
            {
                if (!Path.GetFileName(file).Contains("error"))
                {
                    yield return new object[] { file };
                }
            }
        }

        [Theory]
        [MemberData(nameof(GetPositiveTestFiles))]
        public void Test_All_Positive_Pilot_Scripts_Parse_Cleanly(string filePath)
        {
            string source = File.ReadAllText(filePath);
            var (tree, errors) = ParserHelper.Parse(source);

            Assert.True(errors.Count == 0,
                $"File '{Path.GetFileName(filePath)}' failed with {errors.Count} error(s):\n" + string.Join("\n", errors));
            Assert.NotNull(tree);
        }

        [Fact]
        public void Test_Negative_Syntax_Errors_Are_Accurately_Caught()
        {
            string badCode = @"
def broken_function() {
    let unclosed = (1, 2, 3
";
            var (tree, errors) = ParserHelper.Parse(badCode);
            Assert.NotEmpty(errors);
        }
    }
}
