//! The `iterator` core library module

pub mod adaptors;
pub mod generators;
pub mod peekable;

use crate::{prelude::*, KIteratorOutput as Output, KotoVm, Result};

/// Initializes the `iterator` core library module
pub fn make_module() -> KMap {
    let result = KMap::with_type("core.iterator");

    result.add_fn("all", |ctx| {
        let expected_error = "an iterable and predicate function";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [predicate]) if predicate.is_callable() => {
                let iterable = iterable.clone();
                let predicate = predicate.clone();

                for output in ctx.vm.make_iterator(iterable)? {
                    let predicate_result = match output {
                        Output::Value(value) => ctx
                            .vm
                            .run_function(predicate.clone(), CallArgs::Single(value)),
                        Output::ValuePair(a, b) => ctx
                            .vm
                            .run_function(predicate.clone(), CallArgs::AsTuple(&[a, b])),
                        Output::Error(error) => return Err(error),
                    };

                    match predicate_result {
                        Ok(KValue::Bool(result)) => {
                            if !result {
                                return Ok(false.into());
                            }
                        }
                        Ok(unexpected) => {
                            return type_error(
                                "a Bool to be returned from the predicate",
                                &unexpected,
                            )
                        }
                        error @ Err(_) => return error,
                    }
                }

                Ok(true.into())
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("any", |ctx| {
        let expected_error = "an iterable and predicate function";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [predicate]) if predicate.is_callable() => {
                let iterable = iterable.clone();
                let predicate = predicate.clone();

                for output in ctx.vm.make_iterator(iterable)? {
                    let predicate_result = match output {
                        Output::Value(value) => ctx
                            .vm
                            .run_function(predicate.clone(), CallArgs::Single(value)),
                        Output::ValuePair(a, b) => ctx
                            .vm
                            .run_function(predicate.clone(), CallArgs::AsTuple(&[a, b])),
                        Output::Error(error) => return Err(error),
                    };

                    match predicate_result {
                        Ok(KValue::Bool(result)) => {
                            if result {
                                return Ok(true.into());
                            }
                        }
                        Ok(unexpected) => {
                            return type_error(
                                "a Bool to be returned from the predicate",
                                &unexpected,
                            )
                        }
                        Err(error) => return Err(error),
                    }
                }

                Ok(false.into())
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("chain", |ctx| {
        let expected_error = "two iterable values";
        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable_a, [iterable_b]) if iterable_b.is_iterable() => {
                let iterable_a = iterable_a.clone();
                let iterable_b = iterable_b.clone();
                let result = KIterator::new(adaptors::Chain::new(
                    ctx.vm.make_iterator(iterable_a)?,
                    ctx.vm.make_iterator(iterable_b)?,
                ));

                Ok(KValue::Iterator(result))
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("chunks", |ctx| {
        let expected_error = "an iterable and a chunk size greater than zero";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [KValue::Number(n)]) => {
                let iterable = iterable.clone();
                let n = *n;
                match adaptors::Chunks::new(ctx.vm.make_iterator(iterable)?, n.into()) {
                    Ok(result) => Ok(KIterator::new(result).into()),
                    Err(e) => runtime_error!("iterator.chunks: {}", e),
                }
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("consume", |ctx| {
        let expected_error = "an iterable value (and optional consumer function)";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                for output in ctx.vm.make_iterator(iterable)? {
                    if let Output::Error(error) = output {
                        return Err(error);
                    }
                }
                Ok(KValue::Null)
            }
            (iterable, [f]) if f.is_callable() => {
                let iterable = iterable.clone();
                let f = f.clone();
                for output in ctx.vm.make_iterator(iterable)? {
                    match output {
                        Output::Value(value) => {
                            ctx.vm.run_function(f.clone(), CallArgs::Single(value))?;
                        }
                        Output::ValuePair(a, b) => {
                            ctx.vm.run_function(f.clone(), CallArgs::AsTuple(&[a, b]))?;
                        }
                        Output::Error(error) => return Err(error),
                    }
                }
                Ok(KValue::Null)
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("count", |ctx| {
        let expected_error = "an iterable";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                let mut result = 0;
                for output in ctx.vm.make_iterator(iterable)? {
                    if let Output::Error(error) = output {
                        return Err(error);
                    }
                    result += 1;
                }
                Ok(KValue::Number(result.into()))
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("each", |ctx| {
        let expected_error = "an iterable and function";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [f]) if f.is_callable() => {
                let iterable = iterable.clone();
                let f = f.clone();
                let result = adaptors::Each::new(
                    ctx.vm.make_iterator(iterable)?,
                    f,
                    ctx.vm.spawn_shared_vm(),
                );

                Ok(KIterator::new(result).into())
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("cycle", |ctx| {
        let expected_error = "an iterable";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                let result = adaptors::Cycle::new(ctx.vm.make_iterator(iterable)?);

                Ok(KIterator::new(result).into())
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("enumerate", |ctx| {
        let expected_error = "an iterable";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                let result = adaptors::Enumerate::new(ctx.vm.make_iterator(iterable)?);
                Ok(KIterator::new(result).into())
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("find", |ctx| {
        let expected_error = "an iterable and a predicate function";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [predicate]) if predicate.is_callable() => {
                let iterable = iterable.clone();
                let predicate = predicate.clone();

                for output in ctx.vm.make_iterator(iterable)?.map(collect_pair) {
                    match output {
                        Output::Value(value) => {
                            match ctx
                                .vm
                                .run_function(predicate.clone(), CallArgs::Single(value.clone()))
                            {
                                Ok(KValue::Bool(result)) => {
                                    if result {
                                        return Ok(value);
                                    }
                                }
                                Ok(unexpected) => {
                                    return type_error(
                                        "a Bool to be returned from the predicate",
                                        &unexpected,
                                    )
                                }
                                Err(error) => return Err(error),
                            }
                        }
                        Output::Error(error) => return Err(error),
                        _ => unreachable!(),
                    }
                }

                Ok(KValue::Null)
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("flatten", |ctx| {
        let expected_error = "an iterable";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                let result = adaptors::Flatten::new(
                    ctx.vm.make_iterator(iterable)?,
                    ctx.vm.spawn_shared_vm(),
                );

                Ok(KIterator::new(result).into())
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("fold", |ctx| {
        let expected_error = "an iterable, initial value, and folding function";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [result, f]) if f.is_callable() => {
                let iterable = iterable.clone();
                let result = result.clone();
                let f = f.clone();
                let mut iter = ctx.vm.make_iterator(iterable)?;

                match iter
                    .borrow_internals(|iterator| {
                        let mut fold_result = result.clone();
                        for value in iterator.map(collect_pair) {
                            match value {
                                Output::Value(value) => {
                                    match ctx.vm.run_function(
                                        f.clone(),
                                        CallArgs::Separate(&[fold_result, value]),
                                    ) {
                                        Ok(result) => fold_result = result,
                                        Err(error) => return Some(Output::Error(error)),
                                    }
                                }
                                Output::Error(error) => return Some(Output::Error(error)),
                                _ => unreachable!(),
                            }
                        }

                        Some(Output::Value(fold_result))
                    })
                    // None is never returned from the closure
                    .unwrap()
                {
                    Output::Value(result) => Ok(result),
                    Output::Error(error) => Err(error),
                    _ => unreachable!(),
                }
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("generate", |ctx| match ctx.args() {
        [f] if f.is_callable() => {
            let result = generators::Generate::new(f.clone(), ctx.vm.spawn_shared_vm());
            Ok(KIterator::new(result).into())
        }
        [KValue::Number(n), f] if f.is_callable() => {
            let result = generators::GenerateN::new(n.into(), f.clone(), ctx.vm.spawn_shared_vm());
            Ok(KIterator::new(result).into())
        }
        unexpected => type_error_with_slice("(Function), or (Number, Function)", unexpected),
    });

    result.add_fn("intersperse", |ctx| {
        let expected_error = "an iterable and a separator";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [separator_fn]) if separator_fn.is_callable() => {
                let iterable = iterable.clone();
                let separator_fn = separator_fn.clone();
                let result = adaptors::IntersperseWith::new(
                    ctx.vm.make_iterator(iterable)?,
                    separator_fn,
                    ctx.vm.spawn_shared_vm(),
                );

                Ok(KIterator::new(result).into())
            }
            (iterable, [separator]) => {
                let iterable = iterable.clone();
                let separator = separator.clone();
                let result = adaptors::Intersperse::new(ctx.vm.make_iterator(iterable)?, separator);

                Ok(KIterator::new(result).into())
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("iter", |ctx| {
        let expected_error = "an iterable";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                Ok(KValue::Iterator(ctx.vm.make_iterator(iterable)?))
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("keep", |ctx| {
        let expected_error = "an iterable and a predicate function";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [predicate]) if predicate.is_callable() => {
                let iterable = iterable.clone();
                let predicate = predicate.clone();
                let result = adaptors::Keep::new(
                    ctx.vm.make_iterator(iterable)?,
                    predicate,
                    ctx.vm.spawn_shared_vm(),
                );
                Ok(KIterator::new(result).into())
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("last", |ctx| {
        let expected_error = "an iterable";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                let mut result = KValue::Null;

                let mut iter = ctx.vm.make_iterator(iterable)?.map(collect_pair);
                for output in &mut iter {
                    match output {
                        Output::Value(value) => result = value,
                        Output::Error(error) => return Err(error),
                        _ => unreachable!(),
                    }
                }

                Ok(result)
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("max", |ctx| {
        let expected_error = "an iterable and an optional key function";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                run_iterator_comparison(ctx.vm, iterable, InvertResult::Yes)
            }
            (iterable, [key_fn]) if key_fn.is_callable() => {
                let iterable = iterable.clone();
                let key_fn = key_fn.clone();
                run_iterator_comparison_by_key(ctx.vm, iterable, key_fn, InvertResult::Yes)
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("min", |ctx| {
        let expected_error = "an iterable and an optional key function";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                run_iterator_comparison(ctx.vm, iterable, InvertResult::No)
            }
            (iterable, [key_fn]) if key_fn.is_callable() => {
                let iterable = iterable.clone();
                let key_fn = key_fn.clone();
                run_iterator_comparison_by_key(ctx.vm, iterable, key_fn, InvertResult::No)
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("min_max", |ctx| {
        let expected_error = "an iterable and an optional key function";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                let mut result = None;

                for iter_output in ctx.vm.make_iterator(iterable)?.map(collect_pair) {
                    match iter_output {
                        Output::Value(value) => {
                            result = Some(match result {
                                Some((min, max)) => (
                                    compare_values(ctx.vm, min, value.clone(), InvertResult::No)?,
                                    compare_values(ctx.vm, max, value, InvertResult::Yes)?,
                                ),
                                None => (value.clone(), value),
                            })
                        }
                        Output::Error(error) => return Err(error),
                        _ => unreachable!(),
                    }
                }

                Ok(result.map_or(KValue::Null, |(min, max)| {
                    KValue::Tuple(vec![min, max].into())
                }))
            }
            (iterable, [key_fn]) if key_fn.is_callable() => {
                let iterable = iterable.clone();
                let key_fn = key_fn.clone();
                let mut result = None;

                for iter_output in ctx.vm.make_iterator(iterable)?.map(collect_pair) {
                    match iter_output {
                        Output::Value(value) => {
                            let key = ctx
                                .vm
                                .run_function(key_fn.clone(), CallArgs::Single(value.clone()))?;
                            let value_and_key = (value, key);

                            result = Some(match result {
                                Some((min_and_key, max_and_key)) => (
                                    compare_values_with_key(
                                        ctx.vm,
                                        min_and_key,
                                        value_and_key.clone(),
                                        InvertResult::No,
                                    )?,
                                    compare_values_with_key(
                                        ctx.vm,
                                        max_and_key,
                                        value_and_key,
                                        InvertResult::Yes,
                                    )?,
                                ),
                                None => (value_and_key.clone(), value_and_key),
                            })
                        }
                        Output::Error(error) => return Err(error),
                        _ => unreachable!(), // value pairs have been collected in collect_pair
                    }
                }

                Ok(result.map_or(KValue::Null, |((min, _), (max, _))| {
                    KValue::Tuple(vec![min, max].into())
                }))
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("next", |ctx| {
        let mut iter = match (ctx.instance(), ctx.args()) {
            // No need to call make_iterator when the argument is already an Iterator
            (Some(KValue::Iterator(i)), []) => i.clone(),
            (Some(iterable), []) | (None, [iterable]) if iterable.is_iterable() => {
                ctx.vm.make_iterator(iterable.clone())?
            }
            (_, unexpected) => return type_error_with_slice("an iterable", unexpected),
        };

        iter_output_to_result(iter.next())
    });

    result.add_fn("next_back", |ctx| {
        let mut iter = match (ctx.instance(), ctx.args()) {
            (Some(KValue::Iterator(i)), []) => i.clone(),
            (Some(iterable), []) | (None, [iterable]) if iterable.is_iterable() => {
                ctx.vm.make_iterator(iterable.clone())?
            }
            (_, unexpected) => return type_error_with_slice("an iterable", unexpected),
        };

        iter_output_to_result(iter.next_back())
    });

    result.add_fn("once", |ctx| match ctx.args() {
        [value] => Ok(KIterator::new(generators::Once::new(value.clone())).into()),
        unexpected => type_error_with_slice("a single value", unexpected),
    });

    result.add_fn("peekable", |ctx| {
        let expected_error = "an iterable";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                Ok(peekable::Peekable::make_value(
                    ctx.vm.make_iterator(iterable)?,
                ))
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("position", |ctx| {
        let expected_error = "an iterable and a predicate function";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [predicate]) if predicate.is_callable() => {
                let iterable = iterable.clone();
                let predicate = predicate.clone();

                for (i, output) in ctx.vm.make_iterator(iterable)?.enumerate() {
                    let predicate_result = match output {
                        Output::Value(value) => ctx
                            .vm
                            .run_function(predicate.clone(), CallArgs::Single(value)),
                        Output::ValuePair(a, b) => ctx
                            .vm
                            .run_function(predicate.clone(), CallArgs::AsTuple(&[a, b])),
                        Output::Error(error) => return Err(error),
                    };

                    match predicate_result {
                        Ok(KValue::Bool(result)) => {
                            if result {
                                return Ok(i.into());
                            }
                        }
                        Ok(unexpected) => {
                            return type_error_with_slice(
                                "a Bool to be returned from the predicate",
                                &[unexpected],
                            )
                        }
                        Err(error) => return Err(error),
                    }
                }

                Ok(KValue::Null)
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("product", |ctx| {
        let (iterable, initial_value) = {
            let expected_error = "an iterable and optional initial value";

            match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
                (iterable, []) => (iterable.clone(), KValue::Number(1.into())),
                (iterable, [initial_value]) => (iterable.clone(), initial_value.clone()),
                (_, unexpected) => return type_error_with_slice(expected_error, unexpected),
            }
        };

        fold_with_operator(ctx.vm, iterable, initial_value, BinaryOp::Multiply)
    });

    result.add_fn("repeat", |ctx| match ctx.args() {
        [value] => {
            let result = generators::Repeat::new(value.clone());
            Ok(KIterator::new(result).into())
        }
        [value, KValue::Number(n)] => {
            let result = generators::RepeatN::new(value.clone(), n.into());
            Ok(KIterator::new(result).into())
        }
        unexpected => type_error_with_slice("(Value), or (Number, Value)", unexpected),
    });

    result.add_fn("reversed", |ctx| {
        let expected_error = "an iterable and non-negative number";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                match adaptors::Reversed::new(ctx.vm.make_iterator(iterable)?) {
                    Ok(result) => Ok(KIterator::new(result).into()),
                    Err(e) => runtime_error!("iterator.reversed: {}", e),
                }
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("skip", |ctx| {
        let expected_error = "an iterable and non-negative number";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [KValue::Number(n)]) if *n >= 0.0 => {
                let iterable = iterable.clone();
                let n = *n;
                let mut iter = ctx.vm.make_iterator(iterable)?;

                for _ in 0..n.into() {
                    if let Some(Output::Error(error)) = iter.next() {
                        return Err(error);
                    }
                }

                Ok(KValue::Iterator(iter))
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("step", |ctx| {
        let expected_error = "an iterable and positive step size";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [KValue::Number(n)]) if *n > 0 => {
                let iterable = iterable.clone();
                let step_size = n.into();
                match adaptors::Step::new(ctx.vm.make_iterator(iterable)?, step_size) {
                    Ok(result) => Ok(KIterator::new(result).into()),
                    Err(e) => runtime_error!("iterator.step: {}", e),
                }
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("sum", |ctx| {
        let (iterable, initial_value) = {
            let expected_error = "an iterable and optional initial value";

            match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
                (iterable, []) => (iterable.clone(), KValue::Number(0.into())),
                (iterable, [initial_value]) => (iterable.clone(), initial_value.clone()),
                (_, unexpected) => return type_error_with_slice(expected_error, unexpected),
            }
        };

        fold_with_operator(ctx.vm, iterable, initial_value, BinaryOp::Add)
    });

    result.add_fn("take", |ctx| {
        let expected_error = "an iterable and a count or predicate";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [KValue::Number(n)]) if *n >= 0.0 => {
                let iterable = iterable.clone();
                let n = *n;
                let result = adaptors::Take::new(ctx.vm.make_iterator(iterable)?, n.into());
                Ok(KIterator::new(result).into())
            }
            (iterable, [predicate]) if predicate.is_callable() => {
                let iterable = iterable.clone();
                let predicate = predicate.clone();
                let result = adaptors::TakeWhile::new(
                    ctx.vm.make_iterator(iterable)?,
                    predicate,
                    ctx.vm.spawn_shared_vm(),
                );
                Ok(KIterator::new(result).into())
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("to_list", |ctx| {
        let expected_error = "an iterable";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                let iterator = ctx.vm.make_iterator(iterable)?;
                let (size_hint, _) = iterator.size_hint();
                let mut result = ValueVec::with_capacity(size_hint);

                for output in iterator.map(collect_pair) {
                    match output {
                        Output::Value(value) => result.push(value),
                        Output::Error(error) => return Err(error),
                        _ => unreachable!(),
                    }
                }

                Ok(KValue::List(KList::with_data(result)))
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("to_map", |ctx| {
        let expected_error = "an iterable";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                let iterator = ctx.vm.make_iterator(iterable)?;
                let (size_hint, _) = iterator.size_hint();
                let mut result = ValueMap::with_capacity(size_hint);

                for output in iterator {
                    let (key, value) = match output {
                        Output::ValuePair(key, value) => (key, value),
                        Output::Value(KValue::Tuple(t)) if t.len() == 2 => {
                            let key = t[0].clone();
                            let value = t[1].clone();
                            (key, value)
                        }
                        Output::Value(value) => (value, KValue::Null),
                        Output::Error(error) => return Err(error),
                    };

                    result.insert(ValueKey::try_from(key)?, value);
                }

                Ok(KValue::Map(KMap::with_data(result)))
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("to_string", |ctx| {
        let expected_error = "an iterable";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                let iterator = ctx.vm.make_iterator(iterable)?;
                let (size_hint, _) = iterator.size_hint();
                let mut display_context = DisplayContext::with_vm_and_capacity(ctx.vm, size_hint);
                for output in iterator.map(collect_pair) {
                    match output {
                        Output::Value(KValue::Str(s)) => display_context.append(s),
                        Output::Value(value) => value.display(&mut display_context)?,
                        Output::Error(error) => return Err(error),
                        _ => unreachable!(),
                    };
                }

                Ok(display_context.result().into())
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("to_tuple", |ctx| {
        let expected_error = "an iterable";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, []) => {
                let iterable = iterable.clone();
                let iterator = ctx.vm.make_iterator(iterable)?;
                let (size_hint, _) = iterator.size_hint();
                let mut result = Vec::with_capacity(size_hint);

                for output in iterator.map(collect_pair) {
                    match output {
                        Output::Value(value) => result.push(value),
                        Output::Error(error) => return Err(error),
                        _ => unreachable!(),
                    }
                }

                Ok(KValue::Tuple(result.into()))
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("windows", |ctx| {
        let expected_error = "an iterable and a chunnk size greater than zero";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable, [KValue::Number(n)]) => {
                let iterable = iterable.clone();
                let n = *n;
                match adaptors::Windows::new(ctx.vm.make_iterator(iterable)?, n.into()) {
                    Ok(result) => Ok(KIterator::new(result).into()),
                    Err(e) => runtime_error!("iterator.windows: {}", e),
                }
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result.add_fn("zip", |ctx| {
        let expected_error = "an iterable";

        match ctx.instance_and_args(KValue::is_iterable, expected_error)? {
            (iterable_a, [iterable_b]) if iterable_b.is_iterable() => {
                let iterable_a = iterable_a.clone();
                let iterable_b = iterable_b.clone();
                let result = adaptors::Zip::new(
                    ctx.vm.make_iterator(iterable_a)?,
                    ctx.vm.make_iterator(iterable_b)?,
                );
                Ok(KIterator::new(result).into())
            }
            (_, unexpected) => type_error_with_slice(expected_error, unexpected),
        }
    });

    result
}

pub(crate) fn collect_pair(iterator_output: Output) -> Output {
    match iterator_output {
        Output::ValuePair(first, second) => {
            Output::Value(KValue::Tuple(vec![first, second].into()))
        }
        _ => iterator_output,
    }
}

pub(crate) fn iter_output_to_result(iterator_output: Option<Output>) -> Result<KValue> {
    match iterator_output {
        Some(Output::Value(value)) => Ok(value),
        Some(Output::ValuePair(first, second)) => Ok(KValue::Tuple(vec![first, second].into())),
        Some(Output::Error(error)) => Err(error),
        None => Ok(KValue::Null),
    }
}

fn fold_with_operator(
    vm: &mut KotoVm,
    iterable: KValue,
    initial_value: KValue,
    operator: BinaryOp,
) -> Result<KValue> {
    let mut result = initial_value;

    for output in vm.make_iterator(iterable)?.map(collect_pair) {
        match output {
            Output::Value(rhs_value) => {
                result = vm.run_binary_op(operator, result, rhs_value)?;
            }
            Output::Error(error) => return Err(error),
            _ => unreachable!(),
        }
    }

    Ok(result)
}

fn run_iterator_comparison(
    vm: &mut KotoVm,
    iterable: KValue,
    invert_result: InvertResult,
) -> Result<KValue> {
    let mut result: Option<KValue> = None;

    for iter_output in vm.make_iterator(iterable)?.map(collect_pair) {
        match iter_output {
            Output::Value(value) => {
                result = Some(match result {
                    Some(result) => {
                        compare_values(vm, result.clone(), value.clone(), invert_result)?
                    }
                    None => value,
                })
            }
            Output::Error(error) => return Err(error),
            _ => unreachable!(),
        }
    }

    Ok(result.unwrap_or_default())
}

fn run_iterator_comparison_by_key(
    vm: &mut KotoVm,
    iterable: KValue,
    key_fn: KValue,
    invert_result: InvertResult,
) -> Result<KValue> {
    let mut result_and_key: Option<(KValue, KValue)> = None;

    for iter_output in vm.make_iterator(iterable)?.map(collect_pair) {
        match iter_output {
            Output::Value(value) => {
                let key = vm.run_function(key_fn.clone(), CallArgs::Single(value.clone()))?;
                let value_and_key = (value, key);

                result_and_key = Some(match result_and_key {
                    Some(result_and_key) => {
                        compare_values_with_key(vm, result_and_key, value_and_key, invert_result)?
                    }
                    None => value_and_key,
                });
            }
            Output::Error(error) => return Err(error),
            _ => unreachable!(),
        }
    }

    Ok(result_and_key.map_or(KValue::Null, |(value, _)| value))
}

// Compares two values using BinaryOp::Less
//
// Returns the lesser of the two values, unless `invert_result` is set to Yes
fn compare_values(
    vm: &mut KotoVm,
    a: KValue,
    b: KValue,
    invert_result: InvertResult,
) -> Result<KValue> {
    use InvertResult::*;
    use KValue::Bool;

    let comparison_result = vm.run_binary_op(BinaryOp::Less, a.clone(), b.clone())?;

    match (comparison_result, invert_result) {
        (Bool(true), No) => Ok(a),
        (Bool(false), No) => Ok(b),
        (Bool(true), Yes) => Ok(b),
        (Bool(false), Yes) => Ok(a),
        (other, _) => runtime_error!(
            "Expected Bool from '<' comparison, found '{}'",
            other.type_as_string()
        ),
    }
}

// Compares two values using BinaryOp::Less
//
// Returns the lesser of the two values, unless `invert_result` is set to Yes
fn compare_values_with_key(
    vm: &mut KotoVm,
    a_and_key: (KValue, KValue),
    b_and_key: (KValue, KValue),
    invert_result: InvertResult,
) -> Result<(KValue, KValue)> {
    use InvertResult::*;
    use KValue::Bool;

    let comparison_result =
        vm.run_binary_op(BinaryOp::Less, a_and_key.1.clone(), b_and_key.1.clone())?;

    match (comparison_result, invert_result) {
        (Bool(true), No) => Ok(a_and_key),
        (Bool(false), No) => Ok(b_and_key),
        (Bool(true), Yes) => Ok(b_and_key),
        (Bool(false), Yes) => Ok(a_and_key),
        (other, _) => runtime_error!(
            "Expected Bool from '<' comparison, found '{}'",
            other.type_as_string()
        ),
    }
}

#[derive(Clone, Copy)]
enum InvertResult {
    Yes,
    No,
}
