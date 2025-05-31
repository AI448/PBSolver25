#![feature(float_erf)]

mod plbd_watcher;
mod read_opb;

use std::{io::BufReader, usize};

use pb_engine::{
    Analyze, AnalyzeResult, Boolean, CalculatePLBD, CountConstraintView, LinearConstraintTrait,
    LinearConstraintView, Literal, MonadicClause, PBEngine, PBState,
    strengthen_integer_linear_constraint,
};
use plbd_watcher::PLBDWatcher;
use read_opb::{PBProblem, RelationalOperator, read_opb};

enum Status {
    Satisfiable{solution: Vec<Boolean>},
    Unsatisfiable,
    Indefinite,
}

fn main() {
    if let Some(pb_problem) = read_opb(&mut BufReader::new(std::io::stdin())) {
        let status = solve(&pb_problem);
        match status {
            Status::Satisfiable{solution} => {
                println!("s SATISFIABLE");
                print!("v");
                for (index, &value) in solution.iter().enumerate() {
                    match value {
                        Boolean::TRUE => {
                            print!(" x{}", index + 1);
                        },
                        Boolean::FALSE => {
                            print!(" -x{}", index + 1);
                        }
                    }
                }
                println!("");
            }
            Status::Unsatisfiable => {
                println!("s UNSATISFIABLE");
            }
            Status::Indefinite => {
                println!("s UNKNOWN");
            }
        }
    } else {
        println!("s UNSUPPORTED");
    }
}

fn solve(pb_problem: &PBProblem) -> Status {
    let start_time = std::time::Instant::now();

    let mut pb_engine = PBEngine::new(10.0);

    {
        let max_index = pb_problem
            .constraints
            .iter()
            .map(|constraint| {
                constraint
                    .sum
                    .iter()
                    .map(|weighted_term| weighted_term.term.index)
                    .max()
                    .unwrap_or(0)
            })
            .max()
            .unwrap_or(0);

        // eprintln!("number_of_variables={}", max_index);

        // let mut number_of_appearances = Vec::default();
        // number_of_appearances.resize(max_index, 0usize);
        // for constraint in pb_problem.constraints.iter() {
        //     let n = match constraint.relational_operator {
        //         RelationalOperator::GreaterOrEqual => 1,
        //         RelationalOperator::Equal => 2,
        //     };
        //     for weighter_term in constraint.sum.iter() {
        //         number_of_appearances[weighter_term.term.index - 1] += n;
        //     }
        // }
        // let max_number_of_appearances = *number_of_appearances.iter().max().unwrap();
        for i in 0..max_index {
            pb_engine.add_variable_with_initial_value(
                Boolean::FALSE,
                // number_of_appearances[i] as f64 / max_number_of_appearances as f64,
                0.0
            );
        }
    }

    // pb_engine に制約条件を追加
    {
        fn add_constraint(
            pb_engine: &mut PBEngine,
            terms: impl Iterator<Item = (usize, i64)> + Clone,
            lower: i64,
        ) -> Result<(), ()> {
            // 自明に充足される制約であれば何もしない
            let sum_of_negative_coefficients: i128 = terms
                .clone()
                .filter(|&(_, coefficient)| coefficient < 0)
                .map(|(_, coefficient)| coefficient as i128)
                .sum();
            if lower as i128 - sum_of_negative_coefficients <= 0 {
                return Ok(());
            }

            // 項を (Literal, u64) に変換
            let pb_lower = (lower as i128 - sum_of_negative_coefficients) as u128;
            let pb_terms =
                terms
                    .filter(|&(_, coefficient)| coefficient != 0)
                    .map(|(index, coefficient)| {
                        if coefficient > 0 {
                            (
                                Literal::new(index, pb_engine::Boolean::TRUE),
                                coefficient as u64,
                            )
                        } else {
                            (
                                Literal::new(index, pb_engine::Boolean::FALSE),
                                (-coefficient) as u64,
                            )
                        }
                    });

            // TODO 以下の処理は，現状の PBEngine のラッパーを作ってそこで実装したほうが良い
            // そもそも PBConstraint は外に見せない(explain の戻り値の実装だけに使う)ほうがいいかも

            // 実行不可能か
            let sup: u128 = pb_terms
                .clone()
                .filter(|&(literal, _)| !pb_engine.is_false(literal))
                .map(|(_, coefficient)| coefficient as u128)
                .sum();
            if sup < pb_lower {
                return Err(());
            }

            // 制約を追加
            add_integer_linear_constraint(
                pb_engine,
                &LinearConstraintView::new(pb_terms, pb_lower as u64),
                false,
            );

            return Ok(());
        }

        for constraint in pb_problem.constraints.iter() {
            // >=
            let result = add_constraint(
                &mut pb_engine,
                constraint
                    .sum
                    .iter()
                    .map(|weighted_term| (weighted_term.term.index - 1, weighted_term.weight)),
                constraint.rhs,
            );
            if result.is_err() {
                return Status::Unsatisfiable;
            }
            // <=
            if matches!(constraint.relational_operator, RelationalOperator::Equal) {
                let result = add_constraint(
                    &mut pb_engine,
                    constraint
                        .sum
                        .iter()
                        .map(|weighted_term| (weighted_term.term.index - 1, -weighted_term.weight)),
                    -constraint.rhs,
                );
                if result.is_err() {
                    return Status::Unsatisfiable;
                }
            }
        }
    }

    // eprintln!("   RESTART CONFLICT      PLBD     FIXED    #COUNT   #LINEAR      TIME");

    let mut plbd_watcher = PLBDWatcher::new(10, 10000);
    let mut analyzer = Analyze::new(1e-10);
    let mut calculate_plbd = CalculatePLBD::default();

    let mut conflict_count: usize = 0;
    let mut restart_count: usize = 0;
    let mut previous_restart_timestamp = 0;

    // eprintln!(
    //     "{:9} {:9} {:9} {:9} {:9} {:9} {:9}",
    //     restart_count,
    //     conflict_count,
    //     "",
    //     pb_engine.number_of_assignments(),
    //     pb_engine.number_of_count_constraints(),
    //     pb_engine.number_of_integer_linear_constraints(),
    //     start_time.elapsed().as_secs_f64()
    // );

    loop {
        // if start_time.elapsed() > std::time::Duration::from_secs(600) {
        //     return Status::Indefinite;
        // }

        pb_engine.propagate();
        // eprintln!("{}", pb_engine.number_of_assignments());

        if let PBState::Conflict {
            index: conflict_variable,
            explain_keys: conflict_explain_keys,
        } = pb_engine.state()
        {
            conflict_count += 1;

            if pb_engine.decision_level() == 0 {
                return Status::Unsatisfiable;
            }

            pb_engine.update_assignment_probabilities();

            let analyze_result =
                analyzer.call(conflict_variable, conflict_explain_keys, &pb_engine);

            let AnalyzeResult::Backjumpable {
                backjump_level,
                learnt_constraint,
                conflicting_assignments,
            } = analyze_result
            else {
                return Status::Unsatisfiable;
            };

            let plbd = calculate_plbd.calculate(
                learnt_constraint
                    .iter_terms()
                    .map(|(literal, _)| !literal)
                    .filter(|&literal| pb_engine.is_true(literal)),
                &pb_engine,
            );
            plbd_watcher.add(plbd);
            // plbd_watcher.add(pb_engine.decision_level());
            // eprintln!("plbd={} long_term_mean={}, long_term_variance={}, short_term_mean={}, p={}", plbd, plbd_watcher.long_term_average.mean(), plbd_watcher.long_term_average.variance(), plbd_watcher.short_term_average.mean(), plbd_watcher.lower_tail_probability());

            pb_engine.update_conflict_probabilities(conflicting_assignments, backjump_level);

            let conflict_level = pb_engine.decision_level();

            pb_engine.backjump(backjump_level);

            add_integer_linear_constraint(&mut pb_engine, &learnt_constraint, true);

            // if conflict_count % 10000 == 0 {
            //     eprintln!(
            //         "{:9} {:9} {:9.1} {:9} {:9} {:9} {:9}",
            //         restart_count,
            //         conflict_count,
            //         plbd_watcher.long_term_average.mean(),
            //         pb_engine.number_of_fixed(),
            //         pb_engine.number_of_count_constraints(),
            //         pb_engine.number_of_integer_linear_constraints(),
            //         start_time.elapsed().as_secs_f64()
            //     );
            // }
        } else if pb_engine.number_of_assignments() == pb_engine.number_of_variables() {
            for constraint in pb_problem.constraints.iter() {
                let mut lhs = 0;
                for term in constraint.sum.iter() {
                    if pb_engine.get_value(term.term.index - 1) == Boolean::TRUE {
                        lhs += term.weight;
                    }
                }
                assert!(lhs >= constraint.rhs, "{} {}", lhs, constraint.rhs);
                assert!(
                    constraint.relational_operator == RelationalOperator::GreaterOrEqual
                        || lhs <= constraint.rhs,
                    "{} {}",
                    lhs,
                    constraint.rhs
                );
            }
            let solution = (0..pb_engine.number_of_variables()).map(|index| pb_engine.get_value(index)).collect();
            return Status::Satisfiable{solution};
        } else if conflict_count >= previous_restart_timestamp + 10000
            || (conflict_count >= previous_restart_timestamp + 20
                && plbd_watcher.lower_tail_probability() > 0.6)
        {
            restart_count += 1;
            previous_restart_timestamp = conflict_count;

            if pb_engine.decision_level() != 0 {
                pb_engine.backjump(0);
            }

        } else {
            pb_engine.decide();
        }
    }
}

fn add_integer_linear_constraint(
    pb_engine: &mut PBEngine,
    integer_linear_constraint: &impl LinearConstraintTrait<Value = u64>,
    is_learnt: bool,
) {
    if integer_linear_constraint.lower() == 0 {
        return;
    }
    let integer_linear_constraint = strengthen_integer_linear_constraint(integer_linear_constraint);

    if integer_linear_constraint
        .iter_terms()
        .all(|(_, coefficient)| coefficient == 1)
    {
        if integer_linear_constraint.len() == integer_linear_constraint.lower() as usize {
            for (literal, _) in integer_linear_constraint.iter_terms() {
                pb_engine.add_monadic_clause(MonadicClause { literal }, is_learnt);
            }
        } else {
            pb_engine.add_count_constraint(
                CountConstraintView::new(
                    integer_linear_constraint
                        .iter_terms()
                        .map(|(literal, _)| literal),
                    integer_linear_constraint.lower(),
                ),
                is_learnt,
            );
        }
    } else {
        let mut sum_of_unsaturating_coefficients = 0;
        for (_, coefficient) in integer_linear_constraint.iter_terms() {
            if coefficient < integer_linear_constraint.lower() {
                sum_of_unsaturating_coefficients += coefficient;
            }
        }
        if sum_of_unsaturating_coefficients == integer_linear_constraint.lower() {
            // SAT 符号化して追加
            let saturating_literals = integer_linear_constraint
                .iter_terms()
                .filter(|&(_, coefficient)| coefficient >= integer_linear_constraint.lower())
                .map(|(literal, _)| literal);
            let unsaturating_literals = integer_linear_constraint
                .iter_terms()
                .filter(|&(_, coefficient)| coefficient < integer_linear_constraint.lower())
                .map(|(literal, _)| literal);
            for unsaturating_literal in unsaturating_literals {
                pb_engine.add_count_constraint(
                    CountConstraintView::new(
                        saturating_literals
                            .clone()
                            .chain([unsaturating_literal].into_iter()),
                        1,
                    ),
                    is_learnt,
                );
            }
        } else {
            pb_engine.add_integer_linear_constraint(integer_linear_constraint, is_learnt);
        }
    }
}
