(module lib:OLED_headers (layer F.Cu) (tedit 5E1ADAC2)
        (at {{at}}) 

                
        (fp_text reference "{{ref}}" (at 0 0) (layer F.SilkS) hide (effects (font (size 1.27 1.27) (thickness 0.15))))
        (fp_text value OLED (at 0 -7.3) (layer F.Fab) (effects (font (size 1 1) (thickness 0.15))))

        
        (pad 4 thru_hole oval (at 1.6 2.18 270) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask)
        (net {{net_SDA_id}} "{{net_SDA}}"))
        (pad 3 thru_hole oval (at 1.6 4.72 270) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask)
        (net {{net_SCL_id}} "{{net_SCL}}"))
        (pad 2 thru_hole oval (at 1.6 7.26 270) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask)
        (net {{net_VCC_id}} "{{net_VCC}}"))
        (pad 1 thru_hole rect (at 1.6 9.8 270) (size 1.7 1.7) (drill 1) (layers *.Cu *.Mask)
        (net {{net_GND_id}} "{{net_GND}}"))
        )