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
