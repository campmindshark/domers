# Hardware Mapping

Hardware mapping is treated as fixture data, not an implementation detail to rediscover.

Known constants from Spectrum inventory:

- Dome: 190 struts, 71 projection vertices.
- Bar: routed through dome OPC control box 5 in current operator wiring.
- Stage: 48 sides, 3 layers.
- OPC: Spectrum firmware expects `[channel][0][len_hi][len_lo][RGB...]`, without the usual `0xff` prefix.

Future fixture captures must include every logical-to-device mapping used by dome, bar, and stage outputs.
