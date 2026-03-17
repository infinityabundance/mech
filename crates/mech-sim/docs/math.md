# mech-sim Math Notes

`mech-sim` implements a deterministic reduced-order scaffold whose purpose is architectural validation, not full vehicle fidelity. The state is intentionally small so that the energy, power, thermal, and local-buffer interactions are inspectable and reproducible.

## Core state

The simulator evolves:

- `Ep(t)` pulse-layer stored energy in joules.
- `T(t)` aggregate thermal state in kelvin.
- `y(t)` reduced mechanical output coordinate in meters.
- `v(t) = dy/dt` reduced output velocity in meters per second.
- `Elocal_i(t)` for `i = 1..4` limb-local energy buffers in joules.

## Central pulse-layer energy

The paper-level scaffold is:

`dEp/dt = eta_c * Pc - Pa(u, y) - Pl(Ep, T)`

In the local-buffer extension implemented here, the central pulse layer does not always feed the actuator load directly. Instead, it feeds the limb-local layer through bounded transfers:

`dEp/dt = eta_c * Pc - sum_i Ptransfer_i - Pl(Ep, T)`

where:

- `Pc` is continuous recharge power.
- `eta_c` is recharge efficiency.
- `Ptransfer_i` is central-to-limb transfer power after per-limb transfer limits and energy availability are applied.
- `Pl(Ep, T)` is a parasitic loss model.

The loss model used in code is:

`Pl = P_idle_loss + k_storage * (Ep / Ep_max)^2 + k_temp_loss * max(T - Tamb, 0)`

This is deliberately simple and physically interpretable:

- storage loss grows with state of charge,
- hot hardware leaks more,
- there is always a baseline idle burden.

## Local limb buffers

Each limb-local buffer evolves as:

`dElocal_i/dt = Ptransfer_i - Pdraw_i - Plocal_loss`

with:

- `Pdraw_i` the requested local actuator power share,
- `Ptransfer_i <= Ptransfer_limit`,
- `Elocal_i` clipped to `[0, Elocal,max]`.

If `Ptransfer_i + Elocal_i / dt < Pdraw_i`, the delivered actuator power saturates:

`Pdelivered_i = min(Pdraw_i, Ptransfer_i + Elocal_i / dt)`

This is the key architectural extension. It lets the central pulse layer and the local actuation layer separate in time, which is exactly the kind of claim the paper makes but did not previously validate in reduced-order form.

## Actuator power draw

The reduced-order actuator demand is:

`Pa(u, y, v) = P_idle_act + P_peak * u^1.15 + kv * |v| + ky * |y|`

where:

- `u` is a normalized command fraction from the scenario schedule,
- `P_peak` sets the GW-class burst scale,
- `kv` and `ky` provide simple kinematic coupling so actuator demand is not purely schedule-driven.

This is not a high-fidelity electrohydraulic loss map. It is a transparent surrogate that couples power demand to reduced motion.

## Thermal state

The paper-level scaffold is:

`Ct * dT/dt = Qg(Pa, Pl) - Qr(T)`

The implemented heat generation and rejection models are:

`Qg = alpha_act * Pdelivered + alpha_transfer * Ptransfer_total + Pl`

`Qr = k_reject * max(T - Tamb, 0) + k_reject2 * max(T - Tamb, 0)^2`

where:

- `Ct` is aggregate thermal capacity,
- `alpha_act` maps delivered actuator power into heat,
- `alpha_transfer` captures additional hydraulic/electrical transfer heating,
- `Qr(T)` is an affine-plus-quadratic rejection law.

The hover-equivalent scenario intentionally uses a tighter thermal envelope so the aggregate thermal state becomes the active constraint instead of energy depletion.

## Mechanical output

The reduced mechanical dynamics follow:

`M * y_ddot + D(T) * y_dot + K(y, T) * y = G(Ep, T) * u + d(t)`

with:

`D(T) = D0 * sD * (1 + cD * theta(T))`

`K(y, T) = K0 * sK * (1 - cK * theta(T)) * (1 + cY * y^2)`

`G(Ep, T) = G0 * gE(Ep) * gT(T)`

where:

- `theta(T)` is a normalized temperature fraction,
- `sD` and `sK` are sweepable damping and stiffness scales,
- `gE(Ep)` and `gT(T)` are smooth degradation factors bounded below by `min_gain_fraction`.

The code uses smoothstep-shaped gain degradation:

- low `Ep` reduces force authority,
- high `T` reduces force authority,
- high `T` increases damping,
- high `T` softens stiffness.

## Events and thresholds

The simulator emits explicit machine-readable events when:

- `Ep < low_energy_threshold`,
- `T >= thermal_limit`,
- any `Elocal_i < local_buffer_low_threshold`,
- actuator delivery saturates.

These events are written to `events.csv` and folded into `summary.json` and `derived_metrics.csv`.

## Numerical method

The crate uses a fixed-step semi-implicit Euler integrator:

1. evaluate actuator, transfer, loss, and heat flows from the current state,
2. update velocity using current acceleration,
3. update displacement with the new velocity,
4. update energy and thermal states with the same fixed step.

This was chosen over a more elaborate solver because reproducibility and inspectability were prioritized over model complexity.
