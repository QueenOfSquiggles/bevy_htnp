# Bevy HTNP

> Hierarchical Task Network Planning deeply integrated with bevy

## Main Conceit

Hierarchical Task Networks are characterized by having a concrete set of specific primitive tasks that can be compounded into a sequence that affects the world around them. Because of bevy using ECS, individual task primitives can be modelled with a simple, and standard, bevy system. This plugin handles the heavy lifting of organizing those task primitives, loading and unloading specific components associated with those primitives, and cleaning up if something goes wrong.

## Guaranteed Best Documentation

The best documentation will always be the examples and unit tests. The example `basic_htnp` should be an excellent way to show how to get started.
