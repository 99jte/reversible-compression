use bit_vec::BitVec;
use itertools::Itertools;
use rand::Rng;

pub struct Program {
    fredkins: Vec<(usize, usize, usize)>,
    inp_size: usize
}

impl Program {
    pub fn new(inp_size: usize) -> Self {
        Self {
            fredkins: vec![(0, 1, 4), (3, 1, 4), (2, 3, 1)],
            inp_size
        }
    }

    pub fn forward(&self, mut input: BitVec) -> BitVec {
        assert!(input.len() <= self.inp_size);
        while input.len() < self.inp_size {
            input.push(input.len() % 2 == 0);
        }
        for &(switch, g1, g2) in &self.fredkins {
            assert_ne!(switch, g1);
            assert_ne!(switch, g2);
            assert_ne!(g1, g2);
            if input.get(switch).unwrap() {
                let tmp = !input.get(g2).unwrap(); // THIS IS NOT A VANILLA FREDKINS GATE!
                input.set(g2, input.get(g1).unwrap());
                input.set(g1, tmp);
            }
            /*
            // Tofolli gate
            if input[switch] && input[g1] {
                input.set(g2, !input.get(g2).unwrap());
            }*/
        }
        input
    }

    pub fn eval(&self, input: &BitVec) -> i64 {
        if self.fredkins.is_empty() {
            return 0;
        }
        // To determine which strings were "touched", check only those that were altered
        let max_string_touched = self.fredkins.iter().map(|f| std::cmp::max(f.1, f.2)).max().unwrap();
        let res = self.forward(input.clone());
        //println!("I got {}", res);
        assert_eq!(res.len(), self.inp_size);
        //let effective_outupt_size = std::cmp::max(max_string_touched + 1, input.len()); // If there are strings in no way involved, either in input or in calculation, ignore them as we could do the same in a real compression program
        //res.iter().rev().skip(res.len() - effective_outupt_size).take_while(|b| !b).count() as i64 - (self.fredkins.len() / 2) as i64
        // res.iter().take(effective_outupt_size).take_while(|b| !b).count() as i64/* - (self.fredkins.len() / 10) as i64*/
        // Return the number of bits saved/eliminated by compression
        let remainder_size = (self.inp_size - input.len()) as i64;
        res.iter().take(input.len()).take_while(|b| !b).count() as i64 -
            // Now find the length of the "unspoiled" region of alternating 0s and 1s at the right. Subtract the length of that, from the total length of the 0s-and-1s region, to yield the amount that was "spoiled", ie that we'd need to transmit alnog the wire as well
            (remainder_size - res.iter().enumerate().skip(input.len()).rev().take_while(|(p, v)| *v == (p % 2 == 0)).count() as i64)
    }

    pub fn eval_many(&self, inputs: impl IntoIterator<Item = impl std::borrow::Borrow<BitVec>>) -> i64 {
        inputs.into_iter().map(|i| self.eval(i.borrow())).sum()
    }

    fn eval_many_homogenizing(&self, inputs: impl IntoIterator<Item = impl std::borrow::Borrow<BitVec>>) -> i64 {
        let res = inputs.into_iter().map(|i| self.eval(i.borrow())).collect_vec();
        res.iter().sum::<i64>() - (res.iter().max().unwrap() - res.iter().min().unwrap()) * 4
    }


    fn rectify_duplicates(&self, fredkins: &mut (usize, usize, usize)) {
        // If they're all the same that breaks reversibility so... don't allow that
        if fredkins.0 == fredkins.1 || fredkins.1 == fredkins.2 || fredkins.2 == fredkins.0 {
            // Do this in a really awful way
            /*fredkins.1 = (fredkins.0 + 1) % self.inp_size;
            fredkins.2 = (fredkins.0 + 2) % self.inp_size;*/

            // Slightly less awful way?
            while (fredkins.0 == fredkins.1 || fredkins.0 == fredkins.2) {
                fredkins.0 = (fredkins.0 - 1) % self.inp_size;
            }
            while (fredkins.2 == fredkins.0 || fredkins.2 == fredkins.1) {
                fredkins.2 = (fredkins.2 + 1) % self.inp_size;
            }
        }
        assert!(!(fredkins.0 == fredkins.1 || fredkins.1 == fredkins.2 || fredkins.2 == fredkins.0));
        //println!("{}, {}, {}", fredkins.0, fredkins.1, fredkins.2);
    }

    pub fn mutation(&self, mut_rate: f64) -> Self {
        let mut rng = rand::rng();
        let mut fredkins = self.fredkins.clone();


        for _ in 0..rng.sample(rand_distr::Binomial::new(fredkins.len() as u64, mut_rate).unwrap()) {
            // I feel like this should be able to be more compact
            let middle = rng.random_range(0..fredkins.len());
            let size = std::cmp::min(rand::rng().sample(rand_distr::Normal::new(4.0, 4.0).unwrap()) as usize, std::cmp::min(fredkins.len() - middle - 1, middle - 0),
            );
            //println!("Mid is {}, {}", middle, size);


            match rng.random_range(1..5) {
                1 => { // Shift pos of group, vertical
                    let shamt = rand::rng().sample(rand_distr::Normal::new(0.0, 3.0).unwrap()) as i64;
                    for elem in fredkins[middle - size..=middle + size].iter_mut() {
                        for conn in [&mut elem.0, &mut elem.1, &mut elem.2] {
                            *conn = (*conn as i64 + shamt).clamp(0, self.inp_size as i64 - 1) as usize;
                        }
                        self.rectify_duplicates(elem);
                    }
                }
                2 => { // Horizontal copy
                    let shamt = rand::rng().sample(rand_distr::Normal::new(0.0, 10.0 + (size * 4) as f64).unwrap()) as i64;
                    let excerpt = fredkins[middle - size..=middle + size].to_owned();
                    let insertion_point = (middle as i64 + shamt + size as i64 * shamt.signum()).clamp(0, fredkins.len() as i64 - 1) as usize;
                    let tmp = fredkins.splice(insertion_point..=insertion_point, excerpt).next().unwrap();
                    //println!("SHift by {}", shamt);
                    fredkins.insert(insertion_point, tmp); // PUt it back in
                    assert!(!fredkins.is_empty());
                }
                3 => { // Delete
                    if middle - size > 0 && middle + size < fredkins.len() {
                        fredkins.drain(middle - size..=middle + size);
                    }
                    assert!(!fredkins.is_empty());
                }
                4 => { // Individually alter values within
                    let dist = rand_distr::Normal::new(0.0, 8.0).unwrap();
                    for elem in fredkins[middle - size..=middle + size].iter_mut() {
                        for conn in [&mut elem.0, &mut elem.1, &mut elem.2] {
                            *conn = (*conn as i64 + rand::rng().sample(dist) as i64).clamp(0, self.inp_size as i64 - 1) as usize;
                        }
                        self.rectify_duplicates(elem);
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
        Self {fredkins, inp_size: self.inp_size}
    }
    
    pub fn complexity(&self) -> i64 {
        self.fredkins.len() as i64
    }
}