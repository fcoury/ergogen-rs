(module ComboDiode (layer F.Cu) (tedit 5B24D78E)


        (at {{at}})

        
        (fp_text reference "{{ref}}" (at 0 0) (layer F.SilkS) hide (effects (font (size 1.27 1.27) (thickness 0.15))))
        (fp_text value "" (at 0 0) (layer F.SilkS) hide (effects (font (size 1.27 1.27) (thickness 0.15))))
        
        
        (fp_line (start 0.25 0) (end 0.75 0) (layer F.SilkS) (width 0.1))
        (fp_line (start 0.25 0.4) (end -0.35 0) (layer F.SilkS) (width 0.1))
        (fp_line (start 0.25 -0.4) (end 0.25 0.4) (layer F.SilkS) (width 0.1))
        (fp_line (start -0.35 0) (end 0.25 -0.4) (layer F.SilkS) (width 0.1))
        (fp_line (start -0.35 0) (end -0.35 0.55) (layer F.SilkS) (width 0.1))
        (fp_line (start -0.35 0) (end -0.35 -0.55) (layer F.SilkS) (width 0.1))
        (fp_line (start -0.75 0) (end -0.35 0) (layer F.SilkS) (width 0.1))
        (fp_line (start 0.25 0) (end 0.75 0) (layer B.SilkS) (width 0.1))
        (fp_line (start 0.25 0.4) (end -0.35 0) (layer B.SilkS) (width 0.1))
        (fp_line (start 0.25 -0.4) (end 0.25 0.4) (layer B.SilkS) (width 0.1))
        (fp_line (start -0.35 0) (end 0.25 -0.4) (layer B.SilkS) (width 0.1))
        (fp_line (start -0.35 0) (end -0.35 0.55) (layer B.SilkS) (width 0.1))
        (fp_line (start -0.35 0) (end -0.35 -0.55) (layer B.SilkS) (width 0.1))
        (fp_line (start -0.75 0) (end -0.35 0) (layer B.SilkS) (width 0.1))
    
        
        (pad 1 smd rect (at -1.65 0 0) (size 0.9 1.2) (layers F.Cu F.Paste F.Mask) (net {{net_to_id}} "{{net_to}}"))
        (pad 2 smd rect (at 1.65 0 0) (size 0.9 1.2) (layers B.Cu B.Paste B.Mask) (net {{net_from_id}} "{{net_from}}"))
        (pad 1 smd rect (at -1.65 0 0) (size 0.9 1.2) (layers B.Cu B.Paste B.Mask) (net {{net_to_id}} "{{net_to}}"))
        (pad 2 smd rect (at 1.65 0 0) (size 0.9 1.2) (layers F.Cu F.Paste F.Mask) (net {{net_from_id}} "{{net_from}}"))
        
        
        (pad 1 thru_hole rect (at -3.81 0 0) (size 1.778 1.778) (drill 0.9906) (layers *.Cu *.Mask) (net {{net_to_id}} "{{net_to}}"))
        (pad 2 thru_hole circle (at 3.81 0 0) (size 1.905 1.905) (drill 0.9906) (layers *.Cu *.Mask) (net {{net_from_id}} "{{net_from}}"))
    )