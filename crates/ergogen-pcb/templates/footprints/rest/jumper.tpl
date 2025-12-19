(module lib:Jumper (layer F.Cu) (tedit 5E1ADAC2)
        (at {{at}}) 

                
        (fp_text reference "{{ref}}" (at 0 0) (layer F.SilkS) hide (effects (font (size 1.27 1.27) (thickness 0.15))))
        (fp_text value Jumper (at 0 -7.3) (layer F.Fab) (effects (font (size 1 1) (thickness 0.15))))

        
        (pad 1 smd rect (at -0.50038 0 0) (size 0.635 1.143) (layers F.Cu F.Paste F.Mask)
        (clearance 0.1905) (net {{net_from_id}} "{{net_from}}"))
        (pad 2 smd rect (at 0.50038 0 0) (size 0.635 1.143) (layers F.Cu F.Paste F.Mask)
        (clearance 0.1905) (net {{net_to_id}} "{{net_to}}")))