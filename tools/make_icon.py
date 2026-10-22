#!/usr/bin/env python3
"""Pixel-art Star Town launcher icon: 40px artwork, integer 5x nearest scaling.

Keep the vendor's 300px canvas and 200px plate with transparent 50px margins.
Every painted pixel uses a flat palette; no antialiasing or smooth resampling.
"""
import argparse
from pathlib import Path
from PIL import Image, ImageDraw


def build():
    plate = Image.new('RGBA', (40, 40))
    d = ImageDraw.Draw(plate)
    # Stepped corners and broad colour bands, on the same grid as the house.
    d.polygon([(8,0),(31,0),(31,1),(35,1),(35,3),(37,3),(37,5),(39,5),
               (39,34),(37,34),(37,36),(35,36),(35,38),(31,38),(31,39),
               (8,39),(8,38),(4,38),(4,36),(2,36),(2,34),(0,34),
               (0,5),(2,5),(2,3),(4,3),(4,1),(8,1)], fill='#386b53')
    mask = plate.getchannel('A')
    d.rectangle((0,0,39,20), fill='#78af7b')
    d.rectangle((0,21,39,30), fill='#548b60')
    d.rectangle((0,31,39,39), fill='#386b53')
    d.polygon([(17,32),(23,32),(26,39),(14,39)], fill='#c5b18b')
    d.rectangle((10,11,13,18), fill='#f1d9a7')
    d.rectangle((9,10,14,11), fill='#9b583c')
    d.rectangle((8,19,30,32), fill='#f1d9a7')
    d.rectangle((28,19,30,32), fill='#cba879')
    d.rectangle((8,31,30,33), fill='#dbbd88')
    d.polygon([(5,19),(18,6),(20,6),(34,19),(34,21),(30,21),(19,11),
               (9,21),(5,21)], fill='#674634')
    d.polygon([(5,18),(18,5),(20,5),(34,18),(34,19),(30,19),(19,9),
               (9,19),(5,19)], fill='#cc794b')
    d.line([(9,16),(19,7),(29,16)], fill='#eea565', width=1)
    for x in (10,24):
        d.rectangle((x,22,x+5,27), fill='#976e4d')
        d.rectangle((x+1,23,x+4,26), fill='#83b8bd')
        d.point((x+1,23), fill='#d8e7cf')
        d.rectangle((x-1,28,x+6,28), fill='#bb8a57')
    d.rectangle((18,25,22,32), fill='#885638')
    d.point((21,29), fill='#ffd475')
    d.rectangle((16,33,24,34), fill='#f1d9a7')
    # Hand-stepped star silhouette with a one-pixel shadow.
    star=[(30,3),(32,8),(37,8),(33,12),(34,17),(30,14),(26,17),
          (27,12),(23,8),(28,8)]
    d.polygon([(x,y+1) for x,y in star],fill='#976238')
    d.polygon(star,fill='#ffd475')
    d.line([(30,5),(30,11),(27,13)],fill='#fff0b5',width=1)
    for x in (8,29):
        d.rectangle((x-2,32,x+2,33),fill='#274f42')
        d.rectangle((x-1,30,x+1,31),fill='#83b267')
        d.point((x,30),fill='#ffd475')
    plate.putalpha(mask)
    icon = Image.new('RGBA',(60,60))
    icon.alpha_composite(plate,(10,10))
    return icon.resize((300,300),Image.Resampling.NEAREST)


def main(out_path):
    target=Path(out_path);target.parent.mkdir(parents=True,exist_ok=True)
    icon=build();icon.save(target,'PNG')
    assert icon.getchannel('A').getbbox()==(50,50,250,250)
    assert set(icon.getchannel('A').get_flattened_data())=={0,255}
    print(f'wrote {target} (40px pixel artwork, 5x nearest scaling)')


if __name__=='__main__':
    parser=argparse.ArgumentParser(description=__doc__)
    parser.add_argument('output');main(parser.parse_args().output)
