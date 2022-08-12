# Design

This document outlines some of the design choices I've made with this keyboard, as it is one of my more ambitious designs thus far.

It's a bit more like a rambly blog than anything else.

## Motive and inspiration
I really wanted to play around with low choc profile switches in an ortholinear board, and I wanted to see if the portability of a low profile board would be more convinient than my other current boards.

The three big inspirations (among many others) for this design were the Ghoul, the Technik, the Lumberjack.

The keezyboost40 has the microcontroller in the center, which allows me to fit a microcontroller onto the board without gaining any additional height that would come with, say, soldering the microcontroller under the board.

The acrylic screen and the LCD help make use of the realestate made by having the microcontroller in the center.

## Part choice
I found that generally speaking, the parts I picked generally complemented each other, somewhat by accident. 

I initially picked the rapsberry pi pico over a pro micro due to its lower cost and castelated pins, which I was hoping to use to make the board low profile. However the pico took up considerably more space than the pro-micro. To make use of the real estate, I wanted to do what the Ghoul did, and make use of an LCD screen in the middle. This turned out to be a great compliment, as the large amount of computing power and pins required by the LCD were easily satisfied by the pico. Due to the large amount of pins of the pico, no IO expander or the like is required to have a display _and_ a standard key matrix. This combination of large display and powerful (compared to the ATMega line, at least) microcontroller opens up the door to different kinds of software, UI, and games. I'm excited as this board, on paper, should be capable of doing more things that wasn't previously feasible with a typical OLED and pro-micro. Some ideas I had in mind includes making games or even some sort of tamagotchi program. 

## Firmware design choice
This is arguably where I'm the most torn (and most likely to garner critisism from an online stranger), though this isn't necesarily a one way door either. I needed a firmware that could perform keybaord duties, display to an LCD, as well as be extensible enough that I can comfortably write user programs on top of it. QMK certainly ticks the first box, and thanks to the work of tzarc, writing to a SPI LCD display like mine should be doable with Quantum Painter. My bigger concern with QMK was extensibility (though I'm sure there's some QMK experts who would disagree). To me, I wasn't fully confident that QMK would always have what I need, espectially for something like a tamagotchi game (clocks? interupts? writing to flash? potentially using the pico's second core? There might be a good answer for all of these with QMK, but I didn't like the fact that it feels like I need to / should rely on the constructs and functions that it offers). I could be wrong about most of this QMK stuff, so constructive conversation is welcome.

The biggest appeals to using a Rust based firmware with the keyberon library was the idea that it felt like I was in control of the firmware, and that if there was any problem that I hit with the RP2040 or with the libraries/framework, that I could go into the code and directly fix it myself, rather than having to work in a monolithic structure of sorts. I also like that I could work with a language that has some of the niceities that came with high level languages of the past, as well as a language that can stop me from causing a whole class of bugs. From my limited experience with it, it's a lot more fun programming with it than with C, by a fair margin. Best of all, it's easy to add a library or add a new task to the RTOS if I ever decide to do something esoteric by keyboard standards.

## PCB cutout design choice
For the most part, the PCB is fairly bog-standard. An interesting design choice, of note, however, was the use of having a giant cutout for the pico in the middle of the board. The point of it is so that you can have the pico face down on top of the PCB _and_ be able to access the reset button. It also helps reduce thickness, to a degree by being able to put the pico on top of the PCB rather than at the bottom.

This cut out idea worked well enough in my case, though there's a few caveats that other board builders should be aware of:
- It does somewhat affect board rigdity
    - While completely usable, and not curled up as I initially feared, intentional flexing of the board does concern me a bit.
    - Soldering on a pico helps give board rigidity, though in theory, you're now putting structural strain on the solder joints (!!!) if the board flexes too much
        - Applying generous amounts of solder helps, and having a rigid case prevents this from being a major concern, but I would have fair doubts about putting in in a gasket mount case or without a case
- There's less PCB space
    - hard to fit in fun stuff like buzzers
    - problem is amplified by the size of the pico
- Wrapping traces around the cutout may introduce inductance
    - it's possible that the leads may form a bit of a coil
    - not enough to wreak havoc, though it my be enough to slow/shift keyboard signals enough that adding delays in your firmware become mandatory