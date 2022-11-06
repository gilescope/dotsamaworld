(Desktop architecture is same except both processes are in one process,
at scale the data source process could be in a centralised
server somewhere for scailability.)

## Data Source process

  1. subscribe to rpc nodes (or potentially smaldot)
  2. recieve blocks back async,
  3. render them to model space
  4. and put them on the queue as RenderUpdates.

## Render process

  1. Get RenderUpdates and put on graphics card.
  2. Handle UI and input.