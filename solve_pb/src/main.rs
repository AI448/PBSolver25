use std::{io::BufReader, usize};

use pb_engine::{
    Analyze, AnalyzeResult, Boolean, CountConstraintView,
    LinearConstraintTrait, LinearConstraintView, Literal, MonadicClause, PBConstraint, PBEngine,
    PBState,
};
use read_opb::{PBProblem, RelationalOperator, read_opb};

mod read_opb;

enum Status {
    Satisfiable,
    Unsatisfiable,
    Indefinite,
}

fn main() {
    let pb_problem = read_opb(&mut BufReader::new(std::io::stdin())).unwrap();

    let status = solve(&pb_problem);
    match status {
        Status::Satisfiable => {
            println!("SATISFIABLE")
        }
        Status::Unsatisfiable => {
            println!("UNSATISFIABLE")
        }
        Status::Indefinite => {
            println!("TIMEOUT")
        }
    }
}

fn solve(pb_problem: &PBProblem) -> Status {
    let start_time = std::time::Instant::now();

    let mut pb_engine = PBEngine::new(1e2);

    let max_index = pb_problem.constraints.iter().map(|constraint| constraint.sum.iter().map(|weighted_term| weighted_term.term.index).max().unwrap_or(0)).max().unwrap_or(0);

    eprintln!("number_of_variables={}", max_index);

    for _ in 0..max_index {
        pb_engine.add_variable_with_initial_value(Boolean::FALSE);
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

    eprintln!("   RESTART CONFLICT   #LINEAR    #COUNT  #MONADIC");

    let mut analyzer = Analyze::new(1e-7);

    let mut conflict_count: usize = 0;
    let mut restart_count: usize = 0;
    let mut previous_restart_timestamp = 0;

    eprintln!("{:9} {:9} {:9} {:9} {:9}",
        restart_count, conflict_count,
        pb_engine.number_of_monadic_clauses(),
        pb_engine.number_of_count_constraints(),
        pb_engine.number_of_integer_linear_constraints()
    );

    loop {

        if start_time.elapsed() > std::time::Duration::from_secs(60) {
            return Status::Indefinite;
        }

        pb_engine.propagate();
        // eprintln!("{}", pb_engine.number_of_assignments());

        if let PBState::Conflict {
            index: conflict_variable,
            explain_keys: conflict_explain_keys,
        } = pb_engine.state()
        {
            conflict_count += 1;

            eprintln!("{:9} {:9} {:9} {:9} {:9}",
                restart_count, conflict_count,
                pb_engine.number_of_monadic_clauses(),
                pb_engine.number_of_count_constraints(),
                pb_engine.number_of_integer_linear_constraints()
            );

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

            pb_engine.update_conflict_probabilities(conflicting_assignments);

            pb_engine.backjump(backjump_level);

            add_integer_linear_constraint(&mut pb_engine, &learnt_constraint);
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

            return Status::Satisfiable;
        } else if conflict_count > previous_restart_timestamp + 100 {
            restart_count += 1;
            previous_restart_timestamp = conflict_count;

            if pb_engine.decision_level() != 0 {
                pb_engine.backjump(0);
            }

            // TODO reduce
        } else {
            let decision_variable = {
                let mut decision_variable = None;
                let mut max_activity = 0.0;
                for index in 0..pb_engine.number_of_variables() {
                    if pb_engine.is_assigned(index) {
                        continue;
                    }
                    // let p = [
                    //     pb_engine.assignment_probability(Literal::new(index, Boolean::FALSE)),
                    //     pb_engine.assignment_probability(Literal::new(index, Boolean::TRUE)),
                    // ];
                    // let q = [
                    //     pb_engine.conflict_probability(Literal::new(index, Boolean::FALSE)),
                    //     pb_engine.conflict_probability(Literal::new(index, Boolean::TRUE)),
                    // ];
                    // let r = [
                    //     if p[0] == 0.0 { 0.0 } else { q[0] / p[0] },
                    //     if p[1] == 0.0 { 0.0 } else { q[1] / p[1] },
                    // ];
                    // let activity = r[0] + r[1];
                    let activity = pb_engine.activity(index);
                    if decision_variable.is_none() ||  activity > max_activity {
                        max_activity = activity;
                        decision_variable = Some(index);
                    }
                }
                decision_variable.unwrap()
            };         
            let decision_value = pb_engine.get_value(decision_variable);
            pb_engine.decide(Literal::new(decision_variable, decision_value));
        }
    }
}

fn add_integer_linear_constraint(
    pb_engine: &mut PBEngine,
    integer_linear_constraint: &impl LinearConstraintTrait<Value = u64>,
) {
    if integer_linear_constraint.lower() == 0 {
        return;
    }

    pb_engine.add_constraint(if integer_linear_constraint.len() == 1 {
        PBConstraint::MonadicClause(MonadicClause {
            literal: integer_linear_constraint.iter_terms().next().unwrap().0,
        })
    } else if integer_linear_constraint
        .iter_terms()
        .all(|(_, coefficient)| coefficient == 1)
    {
        PBConstraint::CountConstraint(CountConstraintView::new(
            integer_linear_constraint
                .iter_terms()
                .map(|(literal, _)| literal),
            integer_linear_constraint.lower(),
        ))
    } else {
        PBConstraint::IntegerLinearConstraint(integer_linear_constraint)
    });
}
