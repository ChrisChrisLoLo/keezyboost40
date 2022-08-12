# Assembly

This is the assembly doc used to outline the steps used to assemble a keezyboost40. For the most part, it's fairly typical, and a similar build process to the Corne or Reviung.

For now, I will outline steps of interest, and leave best techniques (such as smd diode soldering) as an exercise to the reader due to time constraints in my personal life. Down the road, I may add more fleshed out steps and pictures for a better beginner build experience.

# General Tools
For the best experience, at the minimum, I suggest a decent soldering iron, flux, wire cutters, and a pair of tweezers to hold/move the SMD parts. I use lead-free solder for generally philisophical reasons, though many do claim that using lead solder is easier to work with. Hot air stations are likely very nice too, though I haven't used one personally.

# PCB Assembly
Firstly, solder the diodes onto the board. You want the black bar (cathode) on the diode  to face _down_ for rows 1,2, and 3. For row 4, you will want the diodes to face _up_. This may change in future iterations. Once done I like to wiggle the diodes gently with my tweezers to see if there's any loose connections. Don't be too rough, or you may tear off a pad!

After that, you will want to solder the hotswap sockets on. Don't worry if the solder from the socket touches the solder from the diode: it's positioned in a way where this isn't a problem.

Then, you'll want to solder the Raspberry pico on the board. You'll have to be fairly careful with this step. You want the "front" of the pico to lay down on the "front" PCB such that reset button is facing downwards, and the diodes and sockets are downwards. The pico should also be resting on _top_ of the PCB, so that the PCB sandwich roughly looks like `pico -> PCB -> diodes/sockets`.
From there, solder the pico onto the board using the castelated pins. Using headers with the pico will almost certainly lead to interference with the LCD screen. Solder one corner of the pico to position it, and then an opposite to secure it once the postioning looks good.

Generally speaking, you'll want to test the connections before going further. You'll want to flash the pico with your firmware of choice, and then test all of the keys to confirm that they work. You can do this by shorting the hotswap sockets.

After that, you'll then want to solder on the LCD display. You can keep the pin headers on, though you will need to clip off the SD card with a pair of wire cutters. We do this as the there is no room to utilize the SD card reader, and leaving it on leads to interference. You'll want to clip around the traces of the SD card to free it. Once clipped off, you'll want to tape the entire backside of the display to cover the pokey bits, as well as the conductive bits that could potentially short with the pico below it. After that, put the TFT onto the board and solder the header pins in place. I suggest securing the TFT down with tape or prop it up with a small piece of foam to minimize it moving around during the soldering process.

Once you have soldered that on, you're now done with the PCB!

# Board assembly
The assemly of the case/board is fairly simple.

Assuming you have an acrylic screen, you'll want to make the following "sandwich" with your long 12mm M2 screw:
`screw head -> acrylic piece -> M2 spacer (goes right through it) -> PCB -> M2 nut`. This will effectively secure the acrylic piece to the PCB.

Afterwards, screw the PCB to the case with the following "sandwich":
`screw head -> bottom of case -> PCB -> M2 nut`

Once done, put on your switches and keycaps, and you're done!
