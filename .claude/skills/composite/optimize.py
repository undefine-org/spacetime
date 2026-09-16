# /// script
# requires-python = ">=3.11"
# dependencies = ["numpy", "opencv-python", "optuna"]
# ///
"""
Declarative parameter optimization framework.

Define parameters as a dataclass with bounds in metadata, provide a runner
and objective function, and let Optuna find optimal values.

Example:
    @dataclass
    class MyParams:
        x: float = field(default=1.0, metadata={'min': 0, 'max': 10})
        y: int = field(default=5, metadata={'min': 0, 'max': 20})

    optimizer = ParamOptimizer(MyParams, runner_fn, objective_fn)
    best_params, score = optimizer.optimize(n_trials=50, **context)
"""

from dataclasses import dataclass, fields, field
from typing import Callable, TypeVar, Generic, Any, get_origin, get_args
import numpy as np

T = TypeVar('T')


@dataclass
class ParamBounds:
    """Extracted bounds from a parameter field."""
    name: str
    type_: type
    default: Any
    min_val: float | int
    max_val: float | int
    step: float | int | None = None


class ParamOptimizer(Generic[T]):
    """
    Generic parameter optimizer using Optuna.

    Args:
        param_class: A dataclass type with fields containing metadata={'min': ..., 'max': ...}
        runner: Function that takes (params=T, **context) and returns a result (e.g., np.ndarray)
        objective: Function that takes (result, context: dict) and returns a float score (lower=better)
    """

    def __init__(
        self,
        param_class: type[T],
        runner: Callable[..., Any],
        objective: Callable[[Any, dict], float],
    ):
        self.param_class = param_class
        self.runner = runner
        self.objective = objective
        self.bounds = self._extract_bounds()

    def _extract_bounds(self) -> list[ParamBounds]:
        """Extract parameter bounds from dataclass metadata."""
        bounds = []
        for f in fields(self.param_class):
            meta = f.metadata or {}

            # Handle Optional types and get base type
            field_type = f.type
            origin = get_origin(field_type)
            if origin is not None:
                args = get_args(field_type)
                field_type = args[0] if args else field_type

            bounds.append(ParamBounds(
                name=f.name,
                type_=field_type,
                default=f.default if f.default is not dataclass else meta.get('default', 0),
                min_val=meta.get('min', 0),
                max_val=meta.get('max', 100),
                step=meta.get('step', None),
            ))
        return bounds

    def _suggest_params(self, trial) -> T:
        """Generate parameter suggestions from Optuna trial."""
        kwargs = {}
        for b in self.bounds:
            if b.type_ == int:
                step = b.step if b.step is not None else 1
                kwargs[b.name] = trial.suggest_int(b.name, int(b.min_val), int(b.max_val), step=int(step))
            elif b.type_ == float:
                if b.step is not None:
                    kwargs[b.name] = trial.suggest_float(b.name, b.min_val, b.max_val, step=b.step)
                else:
                    kwargs[b.name] = trial.suggest_float(b.name, b.min_val, b.max_val)
            elif b.type_ == bool:
                kwargs[b.name] = trial.suggest_categorical(b.name, [True, False])
            else:
                # Default to float
                kwargs[b.name] = trial.suggest_float(b.name, b.min_val, b.max_val)
        return self.param_class(**kwargs)

    def optimize(
        self,
        n_trials: int = 50,
        timeout: float | None = None,
        show_progress: bool = True,
        verbose: bool = True,
        **context,
    ) -> tuple[T, float]:
        """
        Run optimization.

        Args:
            n_trials: Number of optimization trials
            timeout: Max seconds to run (None = no limit)
            show_progress: Show Optuna progress bar
            verbose: Print trial results
            **context: Additional context passed to runner and objective

        Returns:
            (best_params, best_score)
        """
        import optuna

        # Suppress Optuna logs if not verbose
        if not verbose:
            optuna.logging.set_verbosity(optuna.logging.WARNING)

        best_score = float('inf')

        def objective_fn(trial) -> float:
            nonlocal best_score
            params = self._suggest_params(trial)
            result = self.runner(params=params, **context)
            score = self.objective(result, context)

            if verbose and score < best_score:
                best_score = score
                print(f"Trial {trial.number}: new best score = {score:.6f}")

            return score

        study = optuna.create_study(
            direction='minimize',
            sampler=optuna.samplers.TPESampler(seed=42),
        )

        study.optimize(
            objective_fn,
            n_trials=n_trials,
            timeout=timeout,
            show_progress_bar=show_progress,
        )

        # Reconstruct best params
        best_params = self.param_class(**study.best_params)
        return best_params, study.best_value

    def grid_search(
        self,
        grid: dict[str, list],
        verbose: bool = True,
        **context,
    ) -> tuple[T, float]:
        """
        Exhaustive grid search over specified parameter values.

        Args:
            grid: Dict mapping param names to lists of values to try
            verbose: Print progress
            **context: Additional context

        Returns:
            (best_params, best_score)
        """
        import itertools

        # Build all combinations
        param_names = list(grid.keys())
        param_values = [grid[name] for name in param_names]
        combinations = list(itertools.product(*param_values))

        best_params = None
        best_score = float('inf')

        for i, values in enumerate(combinations):
            kwargs = dict(zip(param_names, values))
            params = self.param_class(**kwargs)

            result = self.runner(params=params, **context)
            score = self.objective(result, context)

            if score < best_score:
                best_score = score
                best_params = params
                if verbose:
                    print(f"[{i+1}/{len(combinations)}] New best: {score:.6f} with {kwargs}")

        return best_params, best_score
