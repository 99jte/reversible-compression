mod fredkins_program;
mod arbitrairy_program;

use std::cmp::max;
use rayon::iter::ParallelIterator;
use bit_vec::BitVec;
use rand::{rng, Rng};
use itertools::Itertools;
use rand::seq::IndexedRandom;
use rayon::iter::IntoParallelIterator;


// What would it mean if the mutations could become mulpitlicative in the same language, as mutations?
// E.g. delete this section of gene, clone this other section - at "phyolgeny" time
// And then perhaps had the ability to expand those out into the genes on which we do evolution - is that meaningfuly/useful? Does this just amount to an encoding of a tree structure?

pub fn eval(forward: impl Fn(BitVec) -> BitVec, input: &BitVec) -> i64 {
    let res = forward(input.clone());
    // It is assumed that res is the untruncated output of the program here, it should be equal in size to the program's internal input size
    /*let remainder_size = (res.len() - input.len()) as i64;
    res.iter().take(input.len()).take_while(|b| !b).count() as i64 -
        // Now find the length of the "unspoiled" region of alternating 0s and 1s at the right. Subtract the length of that, from the total length of the 0s-and-1s region, to yield the amount that was "spoiled", ie that we'd need to transmit alnog the wire as well
        (remainder_size - res.iter().enumerate().skip(input.len()).rev().take_while(|(p, v)| *v == (p % 2 == 0)).count() as i64)*/

    //input.len() as i64 - res.iter().coalesce(|a, b| if a == b {Ok(a)} else {Err((a, b))}).count() as i64

    let zeros = res.iter().take_while(|v| *v == false).count() as i64 + res.iter().rev().take_while(|v| *v == false).count() as i64;
    input.len() as i64 - (res.len() as i64 - zeros)

    /*let x: f64 = res.iter().chunk_by(|i| *i).into_iter().map(|(v, group)| f64::log2(group.collect_vec().len() as f64) + 1f64).sum();
    input.len() as i64 - x as i64*/
}

pub fn eval_many(forward: impl Fn(BitVec) -> BitVec + Copy, inputs: impl IntoIterator<Item = impl std::borrow::Borrow<BitVec>>) -> i64 {
    inputs.into_iter().map(|i| eval(forward, i.borrow())).sum()
}


fn main() {
    let tests = (0..222 as u8).map(|s| [s, s+1,s+2,s+3,s+4,s+5,s+6,s+7,s+8]).map(|v| BitVec::from_bytes(&v)).collect_vec();


    //let mut best = Program::new(120);
    let mut bests = vec![arbitrairy_program::Program::<4, 16>::new(400)];

    let mut i: u64 = 0;
    loop {
        let scores: Vec<(arbitrairy_program::Program<4, 16>, i64)> = (0..500).map(|_| bests.choose(&mut rng()).unwrap().mutation(0.1)).collect_vec().into_iter().chain(bests).
            collect_vec().into_par_iter().map(|p| {
            let score = eval_many(|i| p.forward(i), &tests) - (p.complexity() as i64 * 4);
            (p, score)
        }).collect();
        if i % 100 == 0 {
            println!("{:?}", scores.iter().map(|s| s.1).collect_vec());
        }
        bests = scores.into_iter().sorted_by_key(|(p, score)| -*score).take(10).map(|(p, _)| p).collect_vec();
        if i % 100 == 0 {
            println!("Best {}, did {} (namely {}), got:\n{}",
                     bests[0].complexity(),
                     eval_many(|i| bests[0].forward(i), &tests),
                     tests.iter().map(|t| eval(|i| bests[0].forward(i), &t)).map(|v| format!("{}", v)).intersperse(",".to_string()).collect::<String>(),
                     bests[0].forward(tests[0].clone())
            );
        }
        i += 1;
    }
}

/*

fn main() {
    /*let p = Program {
        fredkins: vec![(1, 5, 6), (1, 2, 3), (2, 4, 6)],
        inp_size: 128
    };
    println!("E eval is {}", p.eval(&BitVec::from_bytes("Hi I am".as_bytes())));

    //let test_strings = ["xxxx", "QQQQ", "    ", "aaaa", "bbbb", "CCCC", "dddd", "eeee", "ffff"];
    let test_strings = ["xxyy", "QQRR", "abb", "bbcc", "CCDD", "ddee", "eeff", "kkll"];
    */
    //let tests = test_strings.map(|s| BitVec::from_bytes(s.as_bytes()));
    let tests = (0..222 as u8).map(|s| [s, s+1,s+2,s+3,s+4]).map(|v| BitVec::from_bytes(&v)).collect_vec();


    //let mut best = Program::new(120);
    let mut bests = vec![fredkins_program::Program::new(400)];

    let mut i: u64 = 0;
    loop {/*
        let best_mut = (0..10000).map(|_| best.mutation(0.01)).
            collect_vec().into_par_iter().max_by_key(|p| p.eval_many(&tests) - p.fredkins.len() as i64/2).unwrap();
        if i % 10 == 0 {
            println!("Best {}, did {} (namely {}), got {}", best_mut.fredkins.len(), best_mut.eval_many(&tests), tests.iter().map(|t| best_mut.eval(&t)).map(|v| format!("{}", v)).intersperse(",".to_string()).collect::<String>(), best_mut.forward(tests[0].clone()));
            if i % 100 == 0 {
                println!("Namely -- {:?}", best_mut.fredkins);
                (0..200).map(|_| best.mutation(0.00)).for_each(|p| print!("{}, ", p.eval_many(&tests)));
                println!();
            }
        }

        let cur_eval = best_mut.eval_many(&tests);
        let prev_eval = best.eval_many(&tests);
        if cur_eval >= prev_eval {
            best = best_mut;
        }*/
        let mut scores: Vec<(fredkins_program::Program, i64)> = (0..5000).map(|_| bests.choose(&mut rng()).unwrap().mutation(0.1)).collect_vec().into_iter().chain(bests).
            collect_vec().into_par_iter().map(|p| {
            let score = p.eval_many(&tests) - (p.complexity() as i64 * 4);
            (p, score)
        }).collect();
        let indexes_and_weights = scores.iter().enumerate().map(|(index, (_, score))| (index, score)).collect_vec();
        /*bests = indexes_and_weights.choose_multiple_weighted(&mut rng(), 10, |(_, score)| max(0, **score).pow(2) as f64).unwrap().
            map(|(i, _)| *i).sorted().rev().map(|i| scores.swap_remove(i)).map(|(p, s)| {/*println!("Sampled score is {}", s);*/ p}).collect_vec(); // "pop" the resultant indexes from scores*/
        //scores.iter().for_each(|(_, score)| println!("Old score {}", score));
        bests = scores.into_iter().sorted_by_key(|(p, score)| -*score).take(10).map(|(p, _)| p).collect_vec();
        if i % 100 == 0 {
            println!("Best {}, did {} (namely {}), got {}",
                     bests[0].complexity(),
                     bests[0].eval_many(&tests),
                     tests.iter().map(|t| bests[0].eval(&t)).map(|v| format!("{}", v)).intersperse(",".to_string()).collect::<String>(),
                     bests[0].forward(tests[0].clone())
            );
            if i % 1000 == 0 {
                //println!("Instructions -- {:?}", bests[0].fredkins);
                //(0..200).map(|_| bests[0].mutation(0.00)).for_each(|p| print!("{}, ", p.eval_many(&tests)));
                //println!();
            }
        }

        i += 1;
    }
}
*/