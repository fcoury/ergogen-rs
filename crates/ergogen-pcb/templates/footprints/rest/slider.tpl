(module E73:SPDT_C128955 (layer F.Cu) (tstamp 5BF2CC3C)

            (at {{at}})

            
            (fp_text reference "{{ref}}" (at 0 0) (layer F.SilkS) hide (effects (font (size 1.27 1.27) (thickness 0.15))))
            (fp_text value "" (at 0 0) (layer F.SilkS) hide (effects (font (size 1.27 1.27) (thickness 0.15))))
            
            
            (fp_line (start 1.95 -1.35) (end -1.95 -1.35) (layer F.SilkS) (width 0.15))
            (fp_line (start 0 -1.35) (end -3.3 -1.35) (layer F.SilkS) (width 0.15))
            (fp_line (start -3.3 -1.35) (end -3.3 1.5) (layer F.SilkS) (width 0.15))
            (fp_line (start -3.3 1.5) (end 3.3 1.5) (layer F.SilkS) (width 0.15))
            (fp_line (start 3.3 1.5) (end 3.3 -1.35) (layer F.SilkS) (width 0.15))
            (fp_line (start 0 -1.35) (end 3.3 -1.35) (layer F.SilkS) (width 0.15))
            
            
            (fp_line (start -1.95 -3.85) (end 1.95 -3.85) (layer Dwgs.User) (width 0.15))
            (fp_line (start 1.95 -3.85) (end 1.95 -1.35) (layer Dwgs.User) (width 0.15))
            (fp_line (start -1.95 -1.35) (end -1.95 -3.85) (layer Dwgs.User) (width 0.15))
            
            
            (pad "" np_thru_hole circle (at 1.5 0) (size 1 1) (drill 0.9) (layers *.Cu *.Mask))
            (pad "" np_thru_hole circle (at -1.5 0) (size 1 1) (drill 0.9) (layers *.Cu *.Mask))

            
            (pad 1 smd rect (at 2.25 2.075 0) (size 0.9 1.25) (layers F.Cu F.Paste F.Mask) (net {{net_from_id}} "{{net_from}}"))
            (pad 2 smd rect (at -0.75 2.075 0) (size 0.9 1.25) (layers F.Cu F.Paste F.Mask) (net {{net_to_id}} "{{net_to}}"))
            (pad 3 smd rect (at -2.25 2.075 0) (size 0.9 1.25) (layers F.Cu F.Paste F.Mask))
            
            
            (pad "" smd rect (at 3.7 -1.1 0) (size 0.9 0.9) (layers F.Cu F.Paste F.Mask))
            (pad "" smd rect (at 3.7 1.1 0) (size 0.9 0.9) (layers F.Cu F.Paste F.Mask))
            (pad "" smd rect (at -3.7 1.1 0) (size 0.9 0.9) (layers F.Cu F.Paste F.Mask))
            (pad "" smd rect (at -3.7 -1.1 0) (size 0.9 0.9) (layers F.Cu F.Paste F.Mask))
        )