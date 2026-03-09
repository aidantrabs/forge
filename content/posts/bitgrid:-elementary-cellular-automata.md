---
title: "bitgrid: elementary cellular automata"
description: "exploring elementary cellular automata from first principles"
date: 2026-03-08
tags: [rust, math]
draft: false
---

## what is a cellular automaton?

a cellular automaton is a discrete computational system. you have a row of cells, each in one of a finite number of states, and a rule that determines how each cell updates based on its neighborhood.

an elementary cellular automaton is the simplest nontrivial case:

- the grid is one-dimensional - a row of cells
- each cell has exactly two states: 0 or 1
- each cell's next state depends on three cells: itself and its two immediate neighbors (left, center, right)

at each time step, every cell reads the triple (left, self, right) and produces a new value. that's the entire system.

## why exactly 256 rules?

this is pure combinatorics.

a neighborhood is a triple of binary values. each value is 0 or 1, so there are:

$$2^3 = 8 \text{ possible neighborhood patterns}$$

the 8 patterns, ordered from 111 down to 000:

| pattern | 111 | 110 | 101 | 100 | 011 | 010 | 001 | 000 |
|---------|-----|-----|-----|-----|-----|-----|-----|-----|
| index   |  7  |  6  |  5  |  4  |  3  |  2  |  1  |  0  |

a rule assigns an output bit (0 or 1) to each of these 8 patterns. a rule is a function:

$$f: \{0,1\}^3 \rightarrow \{0,1\}$$

the number of such functions is:

$$2^8 = 256$$

there are exactly 256 elementary cellular automata. no more, no less.

## binary rule encoding

wolfram's naming scheme is elegant. the rule number, expressed in binary, directly encodes the output table.

take rule 30:

$$30_{10} = 00011110_2$$

each bit corresponds to one neighborhood pattern:

| pattern         | 111 | 110 | 101 | 100 | 011 | 010 | 001 | 000 |
|-----------------|-----|-----|-----|-----|-----|-----|-----|-----|
| bit index       |  7  |  6  |  5  |  4  |  3  |  2  |  1  |  0  |
| rule 30 output  |  0  |  0  |  0  |  1  |  1  |  1  |  1  |  0  |

to compute the output for a neighborhood (l, c, r):

1. interpret the triple as a 3-bit number: $i = l \cdot 4 + c \cdot 2 + r$
2. extract bit $i$ from the rule number: $\text{output} = (\text{rule} \gg i) \;\&\; 1$

the entire rule engine fits in one expression. in rust:

```rust
pub fn apply(&self, left: u8, center: u8, right: u8) -> u8 {
    let index = (left << 2) | (center << 1) | right;
    (self.number >> index) & 1
}
```

two lines. no lookup table, no conditionals. the rule number is the lookup table, and bitwise operations are the query.

## running a simulation

the automaton starts with a row of zeros and a single 1 in the center. each generation, we apply the rule to every cell using its left and right neighbors:

```rust
pub fn step(&mut self) {
    let len = self.cells.len();
    let prev = self.cells.clone();
    for i in 0..len {
        let left = if i == 0 { 0 } else { prev[i - 1] };
        let center = prev[i];
        let right = if i == len - 1 { 0 } else { prev[i + 1] };
        self.cells[i] = self.rule.apply(left, center, right);
    }
}
```

boundary cells see 0 beyond the edge. the previous state is cloned so updates within a generation don't interfere with each other.

## three rules, three behaviors

starting from a single cell, we evolve three rules and get radically different results.

### rule 30 - chaos

```
                    █
                   ███
                  ██  █
                 ██ ████
                ██  █   █
               ██ ████ ███
              ██  █    █  █
             ██ ████  ██████
            ██  █   ███     █
           ██ ████ ██  █   ███
```

rule 30 produces chaotic, aperiodic structure. the left side appears disordered while the right side shows faint regularity. despite being fully deterministic, wolfram conjectured it may function as a pseudorandom number generator. no repeating period has been found in the center column.

### rule 90 - the sierpinski triangle

```
                    █
                   █ █
                  █   █
                 █ █ █ █
                █       █
               █ █     █ █
              █   █   █   █
             █ █ █ █ █ █ █ █
            █               █
           █ █             █ █
```

rule 90 is equivalent to xor(left, right) - the center cell doesn't even matter. this trivial operation produces the sierpinski triangle, a well-known fractal with hausdorff dimension $\log_2(3) \approx 1.585$.

the self-similarity is exact: zoom into any triangular region and you find a smaller copy of the whole pattern. a one-bit local operation generating a fractal is one of the most striking results in cellular automata.

we can verify the xor equivalence exhaustively:

```rust
#[test]
fn rule_90_is_xor() {
    let rule = Rule::new(90);
    for l in 0..=1u8 {
        for c in 0..=1u8 {
            for r in 0..=1u8 {
                assert_eq!(rule.apply(l, c, r), l ^ r);
            }
        }
    }
}
```

all 8 inputs confirm it. the center cell is irrelevant.

### rule 110 - turing completeness

```
                    █
                   ██
                  ███
                 ██ █
                █████
               ██   █
              ███  ██
             ██ █ ███
            ███████ █
           ██     ███
```

rule 110 grows asymmetrically to the left with complex interacting structures. in 2004, matthew cook proved that rule 110 is turing complete - it can simulate any computation given the right initial conditions.

this is one of the most profound results in cellular automata theory. a one-dimensional row of bits with a trivial 8-entry lookup table is capable of universal computation.

## measuring complexity

### population density

population density is the fraction of live cells in a generation:

$$\rho(t) = \frac{1}{N} \sum_{i=0}^{N-1} c_i(t)$$

### shannon entropy

shannon entropy measures the information content of a generation from the frequency of 0s and 1s:

$$H = -\sum_{i} p_i \log_2(p_i)$$

maximum entropy $H = 1.0$ means equal proportions of 0s and 1s. minimum $H = 0.0$ means all cells are in the same state.

### comparing rules

running 100 generations on a 201-cell grid:

| rule | behavior | final density | mean density | final entropy | mean entropy |
|------|----------|---------------|--------------|---------------|--------------|
| 30   | chaotic  | 0.5473        | 0.2585       | 0.9935        | 0.7334       |
| 90   | fractal  | 0.0796        | 0.0622       | 0.4008        | 0.3047       |
| 110  | complex  | 0.2537        | 0.1431       | 0.8171        | 0.5503       |
| 184  | simple   | 0.0050        | 0.0050       | 0.0452        | 0.0452       |
| 0    | trivial  | 0.0000        | 0.0000       | 0.0000        | 0.0005       |
| 255  | trivial  | 1.0000        | 0.9900       | 0.0000        | 0.0005       |

the numbers reveal the behavioral classes:

- rule 30 converges to density ~0.5 with entropy near 1.0 - maximum disorder
- rule 90 maintains low density with periodic entropy oscillations tied to powers of 2
- rule 110 sits between order and chaos - moderate density, high but not maximal entropy
- rules 0, 184, 255 are degenerate - they either die out or saturate immediately

wolfram classified elementary cellular automata into four classes:

1. **class 1** - evolves to a uniform state (rule 0, rule 255)
2. **class 2** - evolves to periodic or stable structures (rule 90)
3. **class 3** - chaotic, aperiodic behavior (rule 30)
4. **class 4** - complex structures, long-lived transients (rule 110)

the boundary between class 3 and class 4 is where computation lives.

## what's next

this is the foundation. from here we can explore:

- spatial entropy using block decomposition rather than global frequency
- mutual information between successive generations
- langton's lambda parameter as a predictor of behavioral class
- the full atlas of all 256 rules

the code is minimal - a rule encoder, a grid stepper, analysis functions, and a renderer. everything follows directly from the mathematics. the entire system is deterministic, pure, and fits in a few hundred lines of rust.

the source code is at [github.com/aidantrabs/bitgrid](https://github.com/aidantrabs/bitgrid).
