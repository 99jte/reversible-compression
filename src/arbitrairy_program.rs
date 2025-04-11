use bit_vec::BitVec;
use itertools::{repeat_n, Itertools};
use rand::Rng;

/// Arbitrairy isomorphic mapping of {bit vecs of inp_size} to itself
/// Technically speaking any program can be one of these, it'd just wildly impractical
#[derive(Clone, Debug)]
struct SBox<const InpSize: usize, const TwoToInpSize: usize> {
    shuffles: [BitVec; TwoToInpSize]
}

impl<const InpSize: usize, const TwoToInpSize: usize> SBox<InpSize, TwoToInpSize> {
    fn new() -> Self {
        assert_eq!(TwoToInpSize, 1 << InpSize);
        Self {
            shuffles: (0..(1<<InpSize)).map(|n: u64| Self::num_to_bitvec(n)).collect_array().unwrap().try_into().unwrap()
        }
    }

    fn forward(&self, input: BitVec) -> BitVec {
        assert!(input.len() <= InpSize);
        self.shuffles[Self::bitvec_to_num(&input) as usize].clone()
    }

    pub fn mutation(&self, mut_rate: f64) -> Self {
        let mut new = self.clone();
        for _ in 0..rand::rng().sample(rand_distr::Binomial::new((1<<InpSize) as u64, mut_rate).unwrap()) {
            let a = rand::rng().random_range(0..new.shuffles.len());
            let b = rand::rng().random_range(0..new.shuffles.len());
            new.shuffles.swap(a, b);
        }
        new
    }

    fn num_to_bitvec(n: u64) -> BitVec {
        let mut v = BitVec::from_bytes(&n.to_be_bytes());
        v = v.split_off(v.len() - InpSize); // Only get the end that we want
        v
    }
    fn bitvec_to_num(bitvec: &BitVec) -> u64 {
        let padded: BitVec = repeat_n(false, 8*8 - bitvec.len()).chain(bitvec).collect();
        u64::from_be_bytes(padded.to_bytes().try_into().unwrap())
    }
}

pub struct Program<const GateSize: usize, const TwoToGateSize: usize> {
    gates: Vec<(SBox<GateSize, TwoToGateSize>, [usize; GateSize])>,
    inp_size: usize
}

impl<const GateSize: usize, const TwoToGateSize: usize> Program<GateSize, TwoToGateSize> {
    pub fn new(inp_size: usize) -> Self {
        Self {
            gates: vec![(SBox::new(), (0..GateSize).collect_vec().try_into().unwrap())],
            inp_size
        }
    }

    pub fn forward(&self, mut input: BitVec) -> BitVec {
        assert!(input.len() <= self.inp_size);
        while input.len() < self.inp_size {
            //input.push(input.len() % 2 == 0);
            input.push(false);
        }
        let mut mem = input; // It was confusing to have it be called "input"
        for (shuf_op, connections) in &self.gates {
            let inp: BitVec = BitVec::from_iter(connections.map(|i| mem[i]));
            let out = shuf_op.forward(inp);
            out.iter().zip_eq(connections).for_each(|(val, &wire)| mem.set(wire, val));
        }
        mem
    }

    fn rectify_duplicates(&self, connections: &mut [usize; GateSize]) {
        // If they're all the same that breaks reversibility so... don't allow that
        for i in 0..GateSize {
            let mut delta: i64 = 1; // Goes in the pattern 1, -2, 3, -4, 5, -6; the sum of wherever you stops probes gradually away from the initial pos
            while connections.iter().filter(|c| **c == connections[i]).count() > 1 {
                //println!("{} {} {} {} {}", connections[i] as i64, connections[i] as i64 + delta, (connections[i] as i64 + delta) % (GateSize as i64), ((connections[i] as i64 + delta) % GateSize as i64) as usize, 80000i64 % 3i64);
                connections[i] = ((connections[i] as i64 + delta).rem_euclid(GateSize as i64)) as usize;
                delta = -(delta + delta.signum());
            }
        }
    }

    pub fn mutation(&self, mut_rate: f64) -> Self {
        let mut rng = rand::rng();
        let mut gates = self.gates.clone();


        for _ in 0..rng.sample(rand_distr::Binomial::new(gates.len() as u64 + 1, mut_rate).unwrap()) {
            // I feel like this should be able to be more compact
            let middle = rng.random_range(0..gates.len());
            let size = std::cmp::min(rand::rng().sample(rand_distr::Normal::new(4.0, 4.0).unwrap()) as usize, std::cmp::min(gates.len() - middle - 1, middle - 0),
            );
            //println!("Mid is {}, {}", middle, size);


            match rng.random_range(1..=5) {
                1 => { // Shift pos of group, vertical
                    let shamt = rand::rng().sample(rand_distr::Normal::new(0.0, 3.0).unwrap()) as i64;
                    for gate in gates[middle - size..=middle + size].iter_mut() {
                        for conn in &mut gate.1 {
                            *conn = (*conn as i64 + shamt).clamp(0, self.inp_size as i64 - 1) as usize;
                        }
                        self.rectify_duplicates(&mut gate.1);
                    }
                }
                2 => { // Horizontal copy of group
                    let shamt = rand::rng().sample(rand_distr::Normal::new(0.0, 10.0 + (size * 4) as f64).unwrap()) as i64;
                    let excerpt = gates[middle - size..=middle + size].to_owned();
                    let insertion_point = (middle as i64 + shamt + size as i64 * shamt.signum()).clamp(0, gates.len() as i64 - 1) as usize;
                    let tmp = gates.splice(insertion_point..=insertion_point, excerpt).next().unwrap();
                    //println!("SHift by {}", shamt);
                    gates.insert(insertion_point, tmp); // PUt it back in
                    assert!(!gates.is_empty());
                    //println!("Gates is {:?}", gates);
                }
                3 => { // Delete group
                    if middle - size > 0 && middle + size < gates.len() {
                        gates.drain(middle - size..=middle + size);
                    }
                    assert!(!gates.is_empty());
                }
                4 => { // Individually alter connections
                    let dist = rand_distr::Normal::new(0.0, 16.0).unwrap();
                    for gate in gates[middle - size..=middle + size].iter_mut() {
                        for conn in &mut gate.1 {
                            *conn = (*conn as i64 + rand::rng().sample(dist) as i64).clamp(0, self.inp_size as i64 - 1) as usize;
                        }
                        self.rectify_duplicates(&mut gate.1);
                    }
                },
                5 => { // Mutate the inside of an the SBoxes
                    for gate in gates[middle - size..=middle + size].iter_mut() {
                        let new_sbox = gate.0.mutation(mut_rate);
                        gate.0 = new_sbox;
                    }
                }
                _ => unreachable!()
            }
            /*
            // EXPERIMENT: only append to end
            { // Horizontal copy
                //let excerpt = fredkins[middle - size..=middle + size].to_owned();
                fredkins.extend_from_within(middle - size..=middle + size);
                assert!(!fredkins.is_empty());
            }*/
        };
        Self { gates, inp_size: self.inp_size}
    }

    pub fn complexity(&self) -> i64 {
        self.gates.len() as i64
    }
}