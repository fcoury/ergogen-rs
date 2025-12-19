(module E73:SW_TACT_ALPS_SKQGABE010 (layer F.Cu) (tstamp 5BF2CC94)

        (descr "Low-profile SMD Tactile Switch, https://www.e-switch.com/product-catalog/tact/product-lines/tl3342-series-low-profile-smt-tact-switch")
        (tags "SPST Tactile Switch")

        (at {{at}})
        
        (fp_text reference "{{ref}}" (at 0 0) (layer F.SilkS) hide (effects (font (size 1.27 1.27) (thickness 0.15))))
        (fp_text value "" (at 0 0) (layer F.SilkS) hide (effects (font (size 1.27 1.27) (thickness 0.15))))
        
        
        (fp_line (start 2.75 1.25) (end 1.25 2.75) (layer B.SilkS) (width 0.15))
        (fp_line (start 2.75 -1.25) (end 1.25 -2.75) (layer B.SilkS) (width 0.15))
        (fp_line (start 2.75 -1.25) (end 2.75 1.25) (layer B.SilkS) (width 0.15))
        (fp_line (start -1.25 2.75) (end 1.25 2.75) (layer B.SilkS) (width 0.15))
        (fp_line (start -1.25 -2.75) (end 1.25 -2.75) (layer B.SilkS) (width 0.15))
        (fp_line (start -2.75 1.25) (end -1.25 2.75) (layer B.SilkS) (width 0.15))
        (fp_line (start -2.75 -1.25) (end -1.25 -2.75) (layer B.SilkS) (width 0.15))
        (fp_line (start -2.75 -1.25) (end -2.75 1.25) (layer B.SilkS) (width 0.15))
        
        
        (pad 1 smd rect (at -3.1 -1.85 0) (size 1.8 1.1) (layers B.Cu B.Paste B.Mask) (net {{net_from_id}} "{{net_from}}"))
        (pad 1 smd rect (at 3.1 -1.85 0) (size 1.8 1.1) (layers B.Cu B.Paste B.Mask) (net {{net_from_id}} "{{net_from}}"))
        (pad 2 smd rect (at -3.1 1.85 0) (size 1.8 1.1) (layers B.Cu B.Paste B.Mask) (net {{net_to_id}} "{{net_to}}"))
        (pad 2 smd rect (at 3.1 1.85 0) (size 1.8 1.1) (layers B.Cu B.Paste B.Mask) (net {{net_to_id}} "{{net_to}}"))
    )