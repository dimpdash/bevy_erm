# Entity Relational Mapper (built on bevy)

A bevy plugin designed to aid database access using an Entity Component System architecture.

Individual user requests are passed into the ECS as events. The systems can use a database query that allows loading of entities in the ECS from a database. Once entities have been modified in memory they are flushed back to the database upong a flush event for that request. Each request opens a new database transaction maintaining isolation between requests. 

```

   ┌────────────────────────────────────────────────────┐
   │                                                    │
   │                     Web Server                     │
   │                                                    │
   │                                                    │
   │    ┌─────────────────┐      ┌─────────────────┐    │
   │    │   Requests      │      │   Responses     │    │
   │    │                 │      │                 │    │
   │    │  ┌───────┐      │      │  ┌───────┐      │    │
   │    │  │       │      │      │  │       │      │    │
   │    │  └───────┘      │      │  └───────┘      │    │
   │    │                 │      │                 │    │
   │    │  ┌───────┐      │      │  ┌───────┐      │    │
   │    │  │       │      │      │  │       │      │    │
   │    │  └───────┘      │      │  └───────┘      │    │
   │    │                 │      │                 │    │
   │    │  ┌───────┐      │      │  ┌───────┐      │    │
   │    │  │       │      │      │  │       │      │    │
   │    │  └───────┘      │      │  └───────┘      │    │
   │    │                 │      │                 │    │
   │    └───┬─────────────┘      └─────────────▲───┘    │
   │        │                                  │        │
   │        │                                  │        │
   └────────┼──────────────────────────────────┼────────┘
            │                                  │
    Requests│                                  │Events
    create  │            ┌────────┐            │create
    events  │            │        │            │response
            │          ┌─┴──────┐ │            │
            │          │        │ │            │
            └──────────► Event  │ ├────────────┘
                       │        │ │
                       │        ├─┘
                       └───┬──▲─┘
                Systems    │  │
                process    │  │
                events     │  │
                       ┌───▼──┴──────────┐
                       │                 │
                   ┌───┴───────────────┐ │
                   │   Business Logic  │ │
                   │      Systems      │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   │ │
                   │                   ├─┘
                   └───┬─────────────▲─┘
                       │             │
             Queries   │             │ Queries
             access the│             │ return
             database  │             │ components
         ┌─────────────▼┐           ┌┴─────────────────────────────────────┐
         │   Database   │ loaded    │  Entities                            │
         │              │ in as     │                                      │
         │              │ components│            ┌────────┐   ┌────────┐   │
         │              ├───────────┤  Entity 1: │ Comp 1 │   │ Comp 2 │   │
         │              │           │     ───────┴────────┴───┴────────┘   │
         │              │           │                                      │
         │              │           │                         ┌────────┐   │
         │              │           │  Entity 2:              │ Comp 2 │   │
         │              │           │    ─────────────────────┴────────┘   │
         └──────────────┘           │                                      │
                                    └──────────────────────────────────────┘

```

# Examples
Found in ./examples