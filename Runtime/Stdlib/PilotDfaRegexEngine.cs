using System;
using System.Collections.Concurrent;
using System.Collections.Generic;
using System.Linq;
using System.Text;
using Pilot.Runtime.Values;

namespace Pilot.Runtime.Stdlib;

/// <summary>
/// Milestone 21.4: Pure, zero-dependency Thompson NFA -> Powerset-Construction Deterministic Finite Automaton (DFA)
/// regular expression compiler and linear-time O(N) non-backtracking matcher.
/// Supports <c>comptime compile_regex("...")</c> freezing into <see cref="PilotFrozenMap"/> <c>.rodata</c> tables.
/// </summary>
public sealed class PilotDfaRegex
{
    private static readonly ConcurrentDictionary<string, PilotDfaRegex> Cache = new(StringComparer.Ordinal);

    public string Pattern { get; }
    public bool AnchoredStart { get; }
    public bool AnchoredEnd { get; }
    public int StateCount => _states.Length;
    public int StartState => 0;

    private readonly DfaState[] _states;
    private readonly int[] _flatAscii128;
    private readonly byte[] _acceptFlags;
    private readonly bool _hasNonAscii;

    private sealed class DfaState
    {
        public readonly int[] AsciiNext = new int[128];
        public readonly List<(CharClassPredicate Pred, int NextState)> NonAsciiTransitions = new();
        public bool IsAccepting;

        public DfaState()
        {
            Array.Fill(AsciiNext, -1);
        }
    }

    private sealed class CharClassPredicate
    {
        public readonly bool Negated;
        public readonly List<(char Lo, char Hi)> Ranges = new();

        public CharClassPredicate(bool negated)
        {
            Negated = negated;
        }

        public bool Matches(char c)
        {
            if (Negated && c == '\n' && Ranges.Count == 0)
            {
                // Wildcard `.` matches any character except '\n'
                return false;
            }

            bool hit = false;
            for (int i = 0; i < Ranges.Count; i++)
            {
                var (lo, hi) = Ranges[i];
                if (c >= lo && c <= hi)
                {
                    hit = true;
                    break;
                }
            }
            return Negated ? !hit : hit;
        }
    }

    private sealed class NfaTransition
    {
        public CharClassPredicate? Matcher; // null => epsilon transition
        public int Target;
    }

    private sealed class NfaState
    {
        public readonly List<NfaTransition> Transitions = new();
    }

    private readonly record struct NfaFragment(int Start, int Accept);

    private PilotDfaRegex(string pattern, bool anchoredStart, bool anchoredEnd, DfaState[] states)
    {
        Pattern = pattern;
        AnchoredStart = anchoredStart;
        AnchoredEnd = anchoredEnd;
        _states = states;

        _flatAscii128 = new int[states.Length * 128];
        _acceptFlags = new byte[states.Length];
        bool nonAscii = false;
        for (int s = 0; s < states.Length; s++)
        {
            Array.Copy(states[s].AsciiNext, 0, _flatAscii128, s * 128, 128);
            _acceptFlags[s] = states[s].IsAccepting ? (byte)1 : (byte)0;
            if (states[s].NonAsciiTransitions.Count > 0)
            {
                nonAscii = true;
            }
        }
        _hasNonAscii = nonAscii;
    }

    public static PilotDfaRegex Compile(string pattern)
    {
        pattern ??= string.Empty;
        return Cache.GetOrAdd(pattern, CompileUncached);
    }

    public PilotValue ToFrozenPilotMap()
    {
        var dict = new Dictionary<PilotValue, PilotValue>
        {
            [PilotValue.FromString("__kind__")] = PilotValue.FromKeyword("regex_dfa"),
            [PilotValue.FromString("pattern")] = PilotValue.FromString(Pattern),
            [PilotValue.FromString("state_count")] = PilotValue.FromInt(StateCount),
            [PilotValue.FromString("start_state")] = PilotValue.FromInt(0),
            [PilotValue.FromString("is_dfa")] = PilotValue.True,
            [PilotValue.FromString("anchored_start")] = PilotValue.FromBool(AnchoredStart),
            [PilotValue.FromString("anchored_end")] = PilotValue.FromBool(AnchoredEnd)
        };

        var acceptingList = new List<PilotValue>(_states.Length);
        for (int i = 0; i < _states.Length; i++)
        {
            acceptingList.Add(PilotValue.FromBool(_states[i].IsAccepting));
        }
        dict[PilotValue.FromString("accepting")] = PilotValue.FromFrozenList(acceptingList);

        return PilotValue.FromFrozenMap(new PilotFrozenMap(dict));
    }

    public static PilotDfaRegex ResolveFromPilotValue(PilotValue dfaOrPattern)
    {
        if (dfaOrPattern.Kind == PilotValueKind.String)
        {
            return Compile(dfaOrPattern.AsString());
        }
        if (dfaOrPattern.Kind == PilotValueKind.Map)
        {
            var map = dfaOrPattern.AsMap();
            if (map.TryGetValue(PilotValue.FromString("pattern"), out var patVal))
            {
                return Compile(patVal.AsString());
            }
            if (map.TryGetValue(PilotValue.FromKeyword("pattern"), out var kwPat))
            {
                return Compile(kwPat.AsString());
            }
        }
        if (dfaOrPattern.Kind == PilotValueKind.Object)
        {
            var obj = dfaOrPattern.AsObject();
            if (obj.NativePayload is PilotDfaRegex nativeDfa)
                return nativeDfa;
            if (obj.Fields.TryGetValue("pattern", out var fPat))
                return Compile(fPat.AsString());
            if (obj.Fields.TryGetValue("_dfa", out var inner))
                return ResolveFromPilotValue(inner);
        }
        return Compile(dfaOrPattern.ToString());
    }

    public bool IsMatch(string text)
    {
        text ??= string.Empty;
        if (AnchoredStart && AnchoredEnd)
        {
            return MatchExactAt(text, 0) == text.Length;
        }
        return TryFindFirst(text, 0, out _, out _);
    }

    public bool TryFindFirst(string text, int startAt, out int matchStart, out int matchEnd)
    {
        text ??= string.Empty;
        if (startAt < 0) startAt = 0;
        if (startAt > text.Length)
        {
            matchStart = -1;
            matchEnd = -1;
            return false;
        }

        int nativeRc = Pilot.Runtime.CppBackend.PilotCppBackend.TryDfaFindUtf16(
            _flatAscii128,
            _acceptFlags,
            _states.Length,
            _hasNonAscii,
            AnchoredStart,
            AnchoredEnd,
            text,
            startAt,
            out matchStart,
            out matchEnd);
        if (nativeRc >= 0)
        {
            return nativeRc == 1;
        }

        int maxStart = AnchoredStart ? (startAt == 0 ? 0 : -1) : text.Length;
        for (int s = startAt; s <= maxStart; s++)
        {
            int end = MatchLongestFrom(text, s);
            if (end >= 0)
            {
                if (!AnchoredEnd || end == text.Length)
                {
                    matchStart = s;
                    matchEnd = end;
                    return true;
                }
            }
        }

        matchStart = -1;
        matchEnd = -1;
        return false;
    }

    public List<(int Start, int End, string Value)> FindAll(string text)
    {
        text ??= string.Empty;
        var results = new List<(int, int, string)>();
        int pos = 0;
        while (pos <= text.Length)
        {
            if (!TryFindFirst(text, pos, out int s, out int e))
                break;

            results.Add((s, e, text.Substring(s, e - s)));
            if (e == s)
            {
                pos = e + 1;
            }
            else
            {
                pos = e;
            }
            if (AnchoredStart)
                break;
        }
        return results;
    }

    public string ReplaceAll(string text, string replacement)
    {
        text ??= string.Empty;
        replacement ??= string.Empty;
        var matches = FindAll(text);
        if (matches.Count == 0)
            return text;

        var sb = new StringBuilder(text.Length + replacement.Length * matches.Count);
        int last = 0;
        foreach (var (s, e, _) in matches)
        {
            if (s > last)
                sb.Append(text, last, s - last);
            sb.Append(replacement);
            last = e;
        }
        if (last < text.Length)
            sb.Append(text, last, text.Length - last);
        return sb.ToString();
    }

    public List<string> Split(string text)
    {
        text ??= string.Empty;
        var matches = FindAll(text);
        if (matches.Count == 0)
            return new List<string> { text };

        var parts = new List<string>(matches.Count + 1);
        int last = 0;
        foreach (var (s, e, _) in matches)
        {
            if (e == s) continue;
            parts.Add(text.Substring(last, s - last));
            last = e;
        }
        parts.Add(text[last..]);
        return parts;
    }

    public unsafe bool TryFindFirstUtf8(
        Pilot.Runtime.NativeMemory.PilotSharedBuffer buf,
        int startAt,
        out int matchStart,
        out int matchEnd)
    {
        int byteLen = (int)buf.ByteLength;
        if (startAt < 0) startAt = 0;
        if (startAt > byteLen)
        {
            matchStart = -1;
            matchEnd = -1;
            return false;
        }

        int nativeRc = Pilot.Runtime.CppBackend.PilotCppBackend.TryDfaFindUtf8(
            _flatAscii128,
            _acceptFlags,
            _states.Length,
            _hasNonAscii,
            AnchoredStart,
            AnchoredEnd,
            (byte*)buf.DataPointer,
            byteLen,
            startAt,
            out matchStart,
            out matchEnd);
        if (nativeRc >= 0)
        {
            return nativeRc == 1;
        }

        string decoded = Encoding.UTF8.GetString(new ReadOnlySpan<byte>((byte*)buf.DataPointer, byteLen));
        return TryFindFirst(decoded, startAt, out matchStart, out matchEnd);
    }

    public bool IsMatchUtf8(Pilot.Runtime.NativeMemory.PilotSharedBuffer buf) =>
        TryFindFirstUtf8(buf, 0, out _, out _);

    private int MatchExactAt(string text, int start)
    {
        int state = 0;
        for (int i = start; i < text.Length; i++)
        {
            state = StepState(state, text[i]);
            if (state < 0) return -1;
        }
        return _states[state].IsAccepting ? text.Length : -1;
    }

    private int MatchLongestFrom(string text, int start)
    {
        int state = 0;
        int lastAccept = _states[0].IsAccepting && (!AnchoredEnd || start == text.Length) ? start : -1;

        for (int i = start; i < text.Length; i++)
        {
            state = StepState(state, text[i]);
            if (state < 0)
                break;
            if (_states[state].IsAccepting)
            {
                int candidateEnd = i + 1;
                if (!AnchoredEnd || candidateEnd == text.Length)
                    lastAccept = candidateEnd;
            }
        }
        return lastAccept;
    }

    private int StepState(int stateIdx, char c)
    {
        var st = _states[stateIdx];
        if (c < 128)
            return st.AsciiNext[c];

        for (int i = 0; i < st.NonAsciiTransitions.Count; i++)
        {
            var (pred, next) = st.NonAsciiTransitions[i];
            if (pred.Matches(c))
                return next;
        }
        return -1;
    }

    #region Regex Parser -> Thompson NFA -> Powerset DFA Construction

    private static PilotDfaRegex CompileUncached(string rawPattern)
    {
        bool anchoredStart = false;
        bool anchoredEnd = false;
        string core = rawPattern;

        if (core.StartsWith('^'))
        {
            anchoredStart = true;
            core = core[1..];
        }
        if (core.Length > 0 && core.EndsWith('$') && !IsEscapedAt(core, core.Length - 1))
        {
            anchoredEnd = true;
            core = core[..^1];
        }

        var builder = new RegexNfaParser(core);
        var frag = builder.ParseRegex();
        var dfaStates = BuildPowersetDfa(builder.States, frag.Start, frag.Accept);
        return new PilotDfaRegex(rawPattern, anchoredStart, anchoredEnd, dfaStates);
    }

    private static bool IsEscapedAt(string s, int index)
    {
        int backslashes = 0;
        for (int i = index - 1; i >= 0 && s[i] == '\\'; i--)
            backslashes++;
        return (backslashes & 1) == 1;
    }

    private sealed class RegexNfaParser
    {
        private readonly string _src;
        private int _pos;
        public readonly List<NfaState> States = new();

        public RegexNfaParser(string src)
        {
            _src = src;
        }

        private int NewState()
        {
            int id = States.Count;
            States.Add(new NfaState());
            return id;
        }

        private void AddEpsilon(int from, int to) =>
            States[from].Transitions.Add(new NfaTransition { Matcher = null, Target = to });

        private void AddMatch(int from, int to, CharClassPredicate pred) =>
            States[from].Transitions.Add(new NfaTransition { Matcher = pred, Target = to });

        public NfaFragment ParseRegex()
        {
            var frag = ParseAlternation();
            return frag;
        }

        private NfaFragment ParseAlternation()
        {
            var left = ParseConcatenation();
            while (_pos < _src.Length && _src[_pos] == '|')
            {
                _pos++;
                var right = ParseConcatenation();
                int s = NewState();
                int a = NewState();
                AddEpsilon(s, left.Start);
                AddEpsilon(s, right.Start);
                AddEpsilon(left.Accept, a);
                AddEpsilon(right.Accept, a);
                left = new NfaFragment(s, a);
            }
            return left;
        }

        private NfaFragment ParseConcatenation()
        {
            var terms = new List<NfaFragment>();
            while (_pos < _src.Length && _src[_pos] != ')' && _src[_pos] != '|')
            {
                terms.Add(ParseQuantified());
            }

            if (terms.Count == 0)
            {
                int s = NewState();
                int a = NewState();
                AddEpsilon(s, a);
                return new NfaFragment(s, a);
            }

            var current = terms[0];
            for (int i = 1; i < terms.Count; i++)
            {
                AddEpsilon(current.Accept, terms[i].Start);
                current = new NfaFragment(current.Start, terms[i].Accept);
            }
            return current;
        }

        private NfaFragment ParseQuantified()
        {
            int atomStartPos = _pos;
            var atom = ParseAtom();
            int atomEndPos = _pos;

            while (_pos < _src.Length)
            {
                char c = _src[_pos];
                if (c == '*')
                {
                    _pos++;
                    if (_pos < _src.Length && _src[_pos] == '?') _pos++; // non-greedy modifier accepted
                    atom = ApplyKleeneStar(atom);
                }
                else if (c == '+')
                {
                    _pos++;
                    if (_pos < _src.Length && _src[_pos] == '?') _pos++;
                    atom = ApplyPlus(atom);
                }
                else if (c == '?')
                {
                    _pos++;
                    if (_pos < _src.Length && _src[_pos] == '?') _pos++;
                    atom = ApplyOptional(atom);
                }
                else if (c == '{' && TryParseRepeatBounds(out int minRep, out int? maxRep))
                {
                    atom = ApplyBoundedRepeat(atomStartPos, atomEndPos, atom, minRep, maxRep);
                }
                else
                {
                    break;
                }
            }
            return atom;
        }

        private bool TryParseRepeatBounds(out int minRep, out int? maxRep)
        {
            minRep = 0;
            maxRep = null;
            int save = _pos;
            _pos++; // '{'

            int startDigits = _pos;
            while (_pos < _src.Length && char.IsDigit(_src[_pos])) _pos++;
            if (_pos == startDigits)
            {
                _pos = save;
                return false;
            }
            minRep = int.Parse(_src[startDigits.._pos]);

            if (_pos < _src.Length && _src[_pos] == '}')
            {
                _pos++;
                maxRep = minRep;
                return true;
            }

            if (_pos < _src.Length && _src[_pos] == ',')
            {
                _pos++;
                int maxDigits = _pos;
                while (_pos < _src.Length && char.IsDigit(_src[_pos])) _pos++;
                if (_pos < _src.Length && _src[_pos] == '}')
                {
                    if (_pos > maxDigits)
                        maxRep = int.Parse(_src[maxDigits.._pos]);
                    else
                        maxRep = null;
                    _pos++;
                    return true;
                }
            }

            _pos = save;
            return false;
        }

        private NfaFragment CloneAtom(int startPos, int endPos)
        {
            int saved = _pos;
            _pos = startPos;
            var clone = ParseAtom();
            _pos = saved;
            return clone;
        }

        private NfaFragment ApplyKleeneStar(NfaFragment inner)
        {
            int s = NewState();
            int a = NewState();
            AddEpsilon(s, inner.Start);
            AddEpsilon(s, a);
            AddEpsilon(inner.Accept, inner.Start);
            AddEpsilon(inner.Accept, a);
            return new NfaFragment(s, a);
        }

        private NfaFragment ApplyPlus(NfaFragment inner)
        {
            int s = NewState();
            int a = NewState();
            AddEpsilon(s, inner.Start);
            AddEpsilon(inner.Accept, inner.Start);
            AddEpsilon(inner.Accept, a);
            return new NfaFragment(s, a);
        }

        private NfaFragment ApplyOptional(NfaFragment inner)
        {
            int s = NewState();
            int a = NewState();
            AddEpsilon(s, inner.Start);
            AddEpsilon(s, a);
            AddEpsilon(inner.Accept, a);
            return new NfaFragment(s, a);
        }

        private NfaFragment ApplyBoundedRepeat(int startPos, int endPos, NfaFragment firstAtom, int minRep, int? maxRep)
        {
            var chain = new List<NfaFragment>();
            if (minRep == 0 && maxRep == 0)
            {
                int s = NewState();
                int a = NewState();
                AddEpsilon(s, a);
                return new NfaFragment(s, a);
            }

            int totalFixed = Math.Min(minRep, 64);
            for (int i = 0; i < totalFixed; i++)
            {
                chain.Add(i == 0 ? firstAtom : CloneAtom(startPos, endPos));
            }

            if (maxRep == null)
            {
                var tailStar = ApplyKleeneStar(totalFixed == 0 ? firstAtom : CloneAtom(startPos, endPos));
                chain.Add(tailStar);
            }
            else
            {
                int extraOpt = Math.Min(Math.Max(0, maxRep.Value - minRep), 64);
                for (int i = 0; i < extraOpt; i++)
                {
                    var optAtom = (totalFixed == 0 && i == 0) ? firstAtom : CloneAtom(startPos, endPos);
                    chain.Add(ApplyOptional(optAtom));
                }
            }

            if (chain.Count == 0)
            {
                int s = NewState();
                int a = NewState();
                AddEpsilon(s, a);
                return new NfaFragment(s, a);
            }

            var acc = chain[0];
            for (int i = 1; i < chain.Count; i++)
            {
                AddEpsilon(acc.Accept, chain[i].Start);
                acc = new NfaFragment(acc.Start, chain[i].Accept);
            }
            return acc;
        }

        private NfaFragment ParseAtom()
        {
            if (_pos >= _src.Length)
            {
                int s = NewState();
                int a = NewState();
                AddEpsilon(s, a);
                return new NfaFragment(s, a);
            }

            char c = _src[_pos];
            if (c == '(')
            {
                _pos++;
                // Optional non-capturing prefix `(?:...)`
                if (_pos + 1 < _src.Length && _src[_pos] == '?' && _src[_pos + 1] == ':')
                    _pos += 2;
                var inner = ParseAlternation();
                if (_pos < _src.Length && _src[_pos] == ')')
                    _pos++;
                return inner;
            }

            if (c == '[')
            {
                var pred = ParseBracketClass();
                return SinglePredicateFragment(pred);
            }

            if (c == '.')
            {
                _pos++;
                var dotPred = new CharClassPredicate(negated: true);
                dotPred.Ranges.Add(('\n', '\n'));
                return SinglePredicateFragment(dotPred);
            }

            if (c == '\\')
            {
                _pos++;
                var pred = ParseEscapePredicate();
                return SinglePredicateFragment(pred);
            }

            // Treat unescaped ^ or $ inside body as zero-width epsilon
            if (c is '^' or '$')
            {
                _pos++;
                int s = NewState();
                int a = NewState();
                AddEpsilon(s, a);
                return new NfaFragment(s, a);
            }

            _pos++;
            var lit = new CharClassPredicate(negated: false);
            lit.Ranges.Add((c, c));
            return SinglePredicateFragment(lit);
        }

        private NfaFragment SinglePredicateFragment(CharClassPredicate pred)
        {
            int s = NewState();
            int a = NewState();
            AddMatch(s, a, pred);
            return new NfaFragment(s, a);
        }

        private CharClassPredicate ParseEscapePredicate()
        {
            if (_pos >= _src.Length)
            {
                var bs = new CharClassPredicate(false);
                bs.Ranges.Add(('\\', '\\'));
                return bs;
            }

            char esc = _src[_pos++];
            switch (esc)
            {
                case 'd':
                {
                    var p = new CharClassPredicate(false);
                    p.Ranges.Add(('0', '9'));
                    return p;
                }
                case 'D':
                {
                    var p = new CharClassPredicate(true);
                    p.Ranges.Add(('0', '9'));
                    return p;
                }
                case 'w':
                {
                    var p = new CharClassPredicate(false);
                    p.Ranges.Add(('a', 'z'));
                    p.Ranges.Add(('A', 'Z'));
                    p.Ranges.Add(('0', '9'));
                    p.Ranges.Add(('_', '_'));
                    return p;
                }
                case 'W':
                {
                    var p = new CharClassPredicate(true);
                    p.Ranges.Add(('a', 'z'));
                    p.Ranges.Add(('A', 'Z'));
                    p.Ranges.Add(('0', '9'));
                    p.Ranges.Add(('_', '_'));
                    return p;
                }
                case 's':
                {
                    var p = new CharClassPredicate(false);
                    p.Ranges.Add((' ', ' '));
                    p.Ranges.Add(('\t', '\r'));
                    return p;
                }
                case 'S':
                {
                    var p = new CharClassPredicate(true);
                    p.Ranges.Add((' ', ' '));
                    p.Ranges.Add(('\t', '\r'));
                    return p;
                }
                case 'n':
                {
                    var p = new CharClassPredicate(false);
                    p.Ranges.Add(('\n', '\n'));
                    return p;
                }
                case 'r':
                {
                    var p = new CharClassPredicate(false);
                    p.Ranges.Add(('\r', '\r'));
                    return p;
                }
                case 't':
                {
                    var p = new CharClassPredicate(false);
                    p.Ranges.Add(('\t', '\t'));
                    return p;
                }
                default:
                {
                    var p = new CharClassPredicate(false);
                    p.Ranges.Add((esc, esc));
                    return p;
                }
            }
        }

        private CharClassPredicate ParseBracketClass()
        {
            _pos++; // '['
            bool negated = false;
            if (_pos < _src.Length && _src[_pos] == '^')
            {
                negated = true;
                _pos++;
            }

            var pred = new CharClassPredicate(negated);
            bool first = true;
            while (_pos < _src.Length && (_src[_pos] != ']' || first))
            {
                first = false;
                if (_src[_pos] == '\\')
                {
                    _pos++;
                    if (_pos < _src.Length)
                    {
                        char e = _src[_pos++];
                        switch (e)
                        {
                            case 'd': pred.Ranges.Add(('0', '9')); continue;
                            case 'w':
                                pred.Ranges.Add(('a', 'z'));
                                pred.Ranges.Add(('A', 'Z'));
                                pred.Ranges.Add(('0', '9'));
                                pred.Ranges.Add(('_', '_'));
                                continue;
                            case 's':
                                pred.Ranges.Add((' ', ' '));
                                pred.Ranges.Add(('\t', '\r'));
                                continue;
                            case 'n': pred.Ranges.Add(('\n', '\n')); continue;
                            case 'r': pred.Ranges.Add(('\r', '\r')); continue;
                            case 't': pred.Ranges.Add(('\t', '\t')); continue;
                            default:
                                char loEsc = e;
                                if (_pos + 1 < _src.Length && _src[_pos] == '-' && _src[_pos + 1] != ']')
                                {
                                    _pos++;
                                    char hiEsc = ReadBracketSingleChar();
                                    pred.Ranges.Add((loEsc <= hiEsc ? loEsc : hiEsc, loEsc <= hiEsc ? hiEsc : loEsc));
                                }
                                else
                                {
                                    pred.Ranges.Add((loEsc, loEsc));
                                }
                                continue;
                        }
                    }
                }

                char lo = _src[_pos++];
                if (_pos + 1 < _src.Length && _src[_pos] == '-' && _src[_pos + 1] != ']')
                {
                    _pos++;
                    char hi = ReadBracketSingleChar();
                    pred.Ranges.Add((lo <= hi ? lo : hi, lo <= hi ? hi : lo));
                }
                else
                {
                    pred.Ranges.Add((lo, lo));
                }
            }

            if (_pos < _src.Length && _src[_pos] == ']')
                _pos++;

            return pred;
        }

        private char ReadBracketSingleChar()
        {
            if (_pos >= _src.Length) return '\0';
            if (_src[_pos] == '\\' && _pos + 1 < _src.Length)
            {
                _pos++;
                char e = _src[_pos++];
                return e switch { 'n' => '\n', 'r' => '\r', 't' => '\t', _ => e };
            }
            return _src[_pos++];
        }
    }

    private static HashSet<int> EpsilonClosure(List<NfaState> nfa, IEnumerable<int> seeds)
    {
        var closure = new HashSet<int>();
        var stack = new Stack<int>();
        foreach (int s in seeds)
        {
            if (closure.Add(s))
                stack.Push(s);
        }

        while (stack.Count > 0)
        {
            int cur = stack.Pop();
            foreach (var tr in nfa[cur].Transitions)
            {
                if (tr.Matcher == null && closure.Add(tr.Target))
                {
                    stack.Push(tr.Target);
                }
            }
        }
        return closure;
    }

    private static string CanonicalSetKey(HashSet<int> set)
    {
        if (set.Count == 0) return string.Empty;
        var arr = set.ToArray();
        Array.Sort(arr);
        return string.Join(',', arr);
    }

    private static DfaState[] BuildPowersetDfa(List<NfaState> nfa, int startNfa, int acceptNfa)
    {
        var dfaList = new List<DfaState>();
        var stateMap = new Dictionary<string, int>(StringComparer.Ordinal);
        var queue = new Queue<(int DfaId, HashSet<int> NfaSet)>();

        var startSet = EpsilonClosure(nfa, new[] { startNfa });
        string startKey = CanonicalSetKey(startSet);
        stateMap[startKey] = 0;
        var dfa0 = new DfaState { IsAccepting = startSet.Contains(acceptNfa) };
        dfaList.Add(dfa0);
        queue.Enqueue((0, startSet));

        const int maxDfaStates = 2048;

        while (queue.Count > 0 && dfaList.Count < maxDfaStates)
        {
            var (dfaId, nfaSet) = queue.Dequeue();
            var dfaState = dfaList[dfaId];

            // 1. Exact ASCII 0..127 transition table via subset construction
            for (int ch = 0; ch < 128; ch++)
            {
                char c = (char)ch;
                var moveSeeds = new List<int>();
                foreach (int ns in nfaSet)
                {
                    foreach (var tr in nfa[ns].Transitions)
                    {
                        if (tr.Matcher != null && tr.Matcher.Matches(c))
                            moveSeeds.Add(tr.Target);
                    }
                }

                if (moveSeeds.Count == 0)
                    continue;

                var targetSet = EpsilonClosure(nfa, moveSeeds);
                string key = CanonicalSetKey(targetSet);
                if (!stateMap.TryGetValue(key, out int nextDfaId))
                {
                    nextDfaId = dfaList.Count;
                    stateMap[key] = nextDfaId;
                    dfaList.Add(new DfaState { IsAccepting = targetSet.Contains(acceptNfa) });
                    queue.Enqueue((nextDfaId, targetSet));
                }
                dfaState.AsciiNext[ch] = nextDfaId;
            }

            // 2. Non-ASCII fallback transition for Unicode runes (> 127), e.g., `.` or `[^...]`
            char sampleUnicode = '\u03B1'; // 'α' representative non-ASCII rune
            var uniSeeds = new List<int>();
            foreach (int ns in nfaSet)
            {
                foreach (var tr in nfa[ns].Transitions)
                {
                    if (tr.Matcher != null && tr.Matcher.Matches(sampleUnicode))
                        uniSeeds.Add(tr.Target);
                }
            }
            if (uniSeeds.Count > 0)
            {
                var targetSet = EpsilonClosure(nfa, uniSeeds);
                string key = CanonicalSetKey(targetSet);
                if (!stateMap.TryGetValue(key, out int nextDfaId))
                {
                    nextDfaId = dfaList.Count;
                    stateMap[key] = nextDfaId;
                    dfaList.Add(new DfaState { IsAccepting = targetSet.Contains(acceptNfa) });
                    queue.Enqueue((nextDfaId, targetSet));
                }
                var nonAsciiPred = new CharClassPredicate(negated: true);
                nonAsciiPred.Ranges.Add(('\0', '\u007F'));
                dfaState.NonAsciiTransitions.Add((nonAsciiPred, nextDfaId));
            }
        }

        return dfaList.ToArray();
    }

    #endregion
}
