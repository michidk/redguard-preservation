# Exit Quotes

On a normal quit, the Glide executable (`RGFX.EXE`) prints a random line from
*The Italian Job* (1969) to the DOS console after its version banner.

## Output

`RGFX.EXE` shuts down its subsystems and then writes to the console:

```text
RedGuard version <version> [build <build>]
Copyright (c)1996-98 Bethesda Softworks, All Rights Reserved.

"<quote>"
```

An ordinary quit exits with status zero and prints the quote. A positive exit
status prints its error report in place of the quote. Exit status −99 suppresses
both the banner and the quote.

The quote is enclosed in double quotes and word-wrapped at spaces to fit the
80-column console.

## Selection

The engine holds 100 quotes in a null-terminated pointer table. Selection
combines the C library `rand()` with the engine's frame counter:

```text
offset = frame_counter & 63
repeat
    index = rand() + offset
until index < 100
```

Because `offset` is fixed for one exit, the result is uniform over entries
`offset` through 99. Entries 64–99 can appear on every exit; entry 0 appears
only when `offset` is zero.

## Quote Table

The text below matches the strings in the executable, including their spelling
and punctuation. Speaker names and stage directions are in square brackets.

| # | Quote |
|---|---|
| 0 | On days like these, when skies are blue.... |
| 1 | And then I hear sweet music float around my head... |
| 2 | As I recall the things we said left unsaid... |
| 3 | It's on days like this, that I remember... |
| 4 | Say 'ello to the big world for me Charlie.... [Charlie] Bye, Bye... |
| 5 | Did you hear that ? He said goodbye Mr Bridger. Well he's going Mr Bridger, Well as long as he's going, that's all I care. |
| 6 | [Charlie] Well, you're the last person I expected to see Laurna |
| 7 | [Shop assistant] You know Charles, you could put all these in a museum |
| 8 | [Charlie] ...And do us a favor Adrian, shorten the sleeves love, I'll not a gorilla |
| 9 | [Charlie] ...Eeerrr, I wonder if you'd hold this for me ? [Garage attendant] Oh yes, of course sir. |
| 10 | [Garage attendant] Yes,... I gather you've been in India for about two years sir. [Charlie] Yes, that's right... shooting tigers |
| 11 | [Garage attendant] Yes,... [counting money] ...you must have shot an awful lot of tigers sir. [Charlie] Yes. I used a machine gun |
| 12 | [Mr Bridger] You've been put up to this haven't you ? You've been bribed to upset my natural rhythm and rrrrruin my health |
| 13 | [Mr Bridger] Croaker, this is MY toilet |
| 14 | [Mr Bridger] Last night Mr Governor, my toilet was broken into. [Governor] Toilet ? [Mr Bridger] Toilet |
| 15 | [Launa] And what would you like. [Charlie] Well, everything ! |
| 16 | [Henchman] Right then Fred. [Fred] Wait a minute... [Girls screaming]. [Fred] Charlie's been caught on the job ! |
| 17 | [Charlie] oh, don't come the ol' moody with me Launa, you know 'ow the game's played |
| 18 | \*Bang\* \*Bang\* \*Bang\* [Launa] It's the law charlie. [Charlie] What'd you tell them. [Launa] Charlie, would I tell them anything ? |
| 19 | [Camp Freddie] But Croaker. [Mr Bridger] Croaker can handle it. He's got everything going for him. |
| 20 | [Miss Peach] Oh yes he was doing it. In the lounge |
| 21 | [Miss Peach] Oh... Oh, something quite obscene with Annette |
| 22 | [Miss Peach] Annette. She was terrified of course. [Charlie] Naturally |
| 23 | [Professor Peach] Of course, I wouldn't want to get Matron into trouble |
| 24 | [Professor Peach] She's big... BIG ! |
| 25 | [Charlie] We are about to do a job in....err...Italy |
| 26 | [Charlie] First, Bill Bailey, he'll be my number two |
| 27 | [Dominic] Hello, Chaps. [Everyone grumbles] [Charlie] All right, all right ! |
| 28 | [Charlie] These chinless wonders will get you out of Turin faster than anyone on four wheels |
| 29 | [Charlie] Better known as big William for very obvious reasons |
| 30 | [Charlie] I know he's got some... err...very funny habits, but make 'im feel at 'ome |
| 31 | [Charlie] Now, it's a very difficult job... and the only way we'll only get through it, is if we all work together as a team...and that means you do everything I say |
| 32 | [Mr Bridger] Does he really need all this equipment ? [Camp Freddie] He say's he does. |
| 33 | [Charlie] No, you're meant to use your brakes Chris. [Chris] terrib....terribly sorry Charles |
| 34 | [Mr Bridger] ..Err...what'd y'mean, they're written off ? [Camp Freddie] A series of accidents Mr Bridger |
| 35 | [Charlie] You're only supposed to blow the bloody doors off ? |
| 36 | [Camp Freddie] It's the old under and over routine.... |
| 37 | [Charlie] Piatza Arthur, Piatza. [Arthur] Sorry Charlie. |
| 38 | [Charlie] That's it....wallop |
| 39 | [Camp Freddie] I've only one thing to say about that Mr Bridger... good luck. |
| 40 | [Mr Bridger] Keats, I think we'd better arrange a funeral. |
| 41 | [Mr Bridger] They're the lot that smashed up my cars aren't they. [Charlie] Practice makes perfect Mr Bridger. [Mr Bridger] You pick 'em, don't you. |
| 42 | [Mr Bridger] ...I also want you to remember, that if you don't come back with the goods, Nellie here will turn in her grave, and likely as not jump right out of it and kick yer teeth in. |
| 43 | [Charlie] Dave. Take the valley road to Turin. Ok. Go. |
| 44 | [Charlie] Big William, take the bus along the main road. |
| 45 | [Professor Peach] Pity people aren't as lovely as flowers isn't it ? |
| 46 | [Caterpillar starts up]....[Sound of E-Type Jag being crunched] |
| 47 | [Charlie] You just lost him his insurance bonus ! [Alto Bene] It cost Beckerman his life. |
| 48 | [Alto Bene] ..Just how d'you think you're going to do it... oh excuse me. [Sound of E-Type Jag crunching] |
| 49 | [Alto Bene] Your car Mr Croaker ? [Charlie] Yes. [Alto Bene laughing] Pretty car. |
| 50 | [Charlie] Very funny. [Alto Bene] Tut tut tut. |
| 51 | [Alto Bene] Well gentlemen. It's a long walk back to England, and it's that way |
| 52 | [Alto Bene] I wouldn't be too sure about the English Cousin. [Francisco Cosca] They wouldn't dare ! [Alto Bene] They're not so stupid as they look. |
| 53 | [Arthur nearly snips Professor Peaches nose off] |
| 54 | [Sound of Billiards falling over] [Alto Bene] Gentlemen, we must assume they are here. |
| 55 | [Charlie] Now, as you walk out to the plane just look straight ahead, look neither to the right, nor to the left. |
| 56 | [Launa] Charlie, listen the plan was that we stay here. [Charlie] Well plans change. |
| 57 | [Charlie] launa, you see that lot out there ? If we slip up, they'll tear us apart. |
| 58 | [Launa] Charlie. [Charlie] What. [Launa] you care ? [Charlie] Get on the plane. |
| 59 | [Charlie] I'll see you in Geneva. 'Ave a cup of tea ready. |
| 60 | [Charlie] Before we leave, bill. [Bill] Yes Charlie. [Charlie] I want every one of those tins burned. |
| 61 | [Bill] We're all wearing our gloves Charlie. [Charlie] yeah, but you can't be too careful. |
| 62 | [Charlie] Right. Here is the crunch. [Points to map] |
| 63 | [Chris] Actually, we can all read maps Charles. [Charlie] The map'll be in your head Chris, I'm burning this one. |
| 64 | [Chris] Shall we synchronize our watches ? [Charlie] Nuts to your watches. |
| 65 | [Charlie] Oh...one more thing, just remember this. In this country they drive on the wrong side of the road. |
| 66 | [Charlie] Bill. [Bill] Yes Charlie. [Charlie] Bill ? [Bill] Yes Charlie. [Charlie] Burn this. [Bill] Yes Charlie. [Charlie] Oh Bill [Bill] yes Charlie. [Charlie] Get rid of this lot. [Bill] Yes Charlie. |
| 67 | [Charlie] You are not gonna get sick, you are not gonna get a migraine, and everyone is gonna sit in the back of the motor. [Arthur] Charlie, me in the back of the motor with my asma. |
| 68 | [Charlie] One more word out of you Arthur. All right ! |
| 69 | [Arthur] ..it was all I had to do... [Charlie] Arthur ! [Arthur] .. all the time....what ? |
| 70 | [Old man] Gratzi. [Charlie] Multibene, dad. |
| 71 | [Fat Englishman] Can you direct me to Calda Garibaldi Street ? [Italian speaks in really fast Italian] [fat Englishman] Bloody foreigners. |
| 72 | [Alto Bene] What's happening ? [Chauffeur] It's another traffic jam, it gets worse every time. |
| 73 | [Charlie] What are they gabbling about Franco. [Franco] Complaining about the traffic jam. [Arthur] Yeah, well they'll 'ave something else to complain about in a minute. |
| 74 | [Charlie bangs the side of the truck] Ok Bill ! |
| 75 | [Everyone creeps back] [Charlie] Whad'y think you lot are doing, don't just panic. Get on with it. Get in the car. Anybody comes through... hit 'em ! |
| 76 | [Charlie] OI ! Get in the Doormobile. |
| 77 | [Bill] Come on Arthur, get your finger out will you. |
| 78 | [Bill] Well look 'appy then you stupid bastards, we won didn't we ? |
| 79 | [Bill mumbles] They went that-away. |
| 80 | This is the self-preservation society... |
| 81 | [Arthur] I could eat a horse.. ha..ah-ah |
| 82 | [Charlie shouts from the car window] GOOD LUCK ! |
| 83 | [Chris] Try putting your foot down Tony, they're really getting rather close. |
| 84 | [Charlie] Now as you go round look for that bloody exit, we can't go round here all night. |
| 85 | [Chris] I think you might make an effort to keep up with the others Tony. [Tony] All right ! |
| 86 | [Arthur holds out the wish-bone] Ah... make a wish. |
| 87 | [Charlie] Look out they're behind us, you'd better put your foot down, put your foot down we'll lose 'em easy. |
| 88 | [Keats] Have you heard the news sir, they've done it Mr Bridger, all the men are cheering for you. They've done it ! |
| 89 | [Mr Bridger] Done what Keats ? [Keats] The job sir ! |
| 90 | Put on your dicky-dirt and your Peckham rye... |
| 91 | [Charlie] Get yer..get yer wheels in line... slam on yer brakes or we'll be in the cabin. |
| 92 | [Dominic] Ok, hold on. [Wheel crunches] [Wheel crunches again] [Wheel crunches yet again] [Chris] Be very careful ! |
| 93 | [Big William] Charlie boy. [Charlie] NOW ! [Car rolls over the cliff] |
| 94 | [Charlie] Ok, out...OUT. Leave the beer. [Arthur] No ! |
| 95 | [Charlie] Hold still...hold still...hold still...hold still. Nobody move. |
| 96 | [Charlie] We're balancing right on the edge. Very slowly move this way. Very slowly ! Don't make any sudden movements. |
| 97 | [Charlie] The gold is pulling it over the edge. We'll 'ave to get it back. |
| 98 | [Charlie creeps forward] [So does everyone else] [Charlie] Get back ! Get back ! Now hold still. Don't move. Don't move at all. Don't no-one get out the door neither. |
| 99 | [Charlie] Hang on a minute lads. I've got a great idea....errr...errr... |

## External References

- [The Italian Job (1969) on Wikipedia](https://en.wikipedia.org/wiki/The_Italian_Job)
