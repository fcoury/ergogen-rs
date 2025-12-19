(module RollerEncoder_Panasonic_EVQWGD001 (layer F.Cu) (tedit 6040A10C)
        (at {{at}})   
        (fp_text reference REF** (at 0 0 0) (layer F.Fab) (effects (font (size 1 1) (thickness 0.15))))
        (fp_text value RollerEncoder_Panasonic_EVQWGD001 (at -0.1 9 0) (layer F.Fab) (effects (font (size 1 1) (thickness 0.15))))
        
        
        (fp_line (start -8.4 -6.4) (end 8.4 -6.4) (layer Dwgs.User) (width 0.12))
        (fp_line (start 8.4 -6.4) (end 8.4 7.4) (layer Dwgs.User) (width 0.12))
        (fp_line (start 8.4 7.4) (end -8.4 7.4) (layer Dwgs.User) (width 0.12))
        (fp_line (start -8.4 7.4) (end -8.4 -6.4) (layer Dwgs.User) (width 0.12))
      
        
          
          (fp_line (start 9.8 7.3) (end 9.8 -6.3) (layer Edge.Cuts) (width 0.15))
          (fp_line (start 7.4 -6.3) (end 7.4 7.3) (layer Edge.Cuts) (width 0.15))
          (fp_line (start 9.5 -6.6) (end 7.7 -6.6) (layer Edge.Cuts) (width 0.15))
          (fp_line (start 7.7 7.6) (end 9.5 7.6) (layer Edge.Cuts) (width 0.15))
          (fp_arc (start 7.7 7.3) (end 7.4 7.3) (angle -90) (layer Edge.Cuts) (width 0.15))
          (fp_arc (start 9.5 7.3) (end 9.5 7.6) (angle -90) (layer Edge.Cuts) (width 0.15))
          (fp_arc (start 7.7 -6.3) (end 7.7 -6.6) (angle -90) (layer Edge.Cuts) (width 0.15))
          (fp_arc (start 9.5 -6.3) (end 9.8 -6.3) (angle -90) (layer Edge.Cuts) (width 0.15))

          
          (pad S1 thru_hole circle (at -6.85 -6.2 0) (size 1.6 1.6) (drill 0.9) (layers *.Cu *.Mask) (net {{net_from_id}} "{{net_from}}"))
          (pad S2 thru_hole circle (at -5 -6.2 0) (size 1.6 1.6) (drill 0.9) (layers *.Cu *.Mask) (net {{net_to_id}} "{{net_to}}"))
          (pad A thru_hole circle (at -5.625 -3.81 0) (size 1.6 1.6) (drill 0.9) (layers *.Cu *.Mask) (net {{net_A_id}} "{{net_A}}"))
          (pad B thru_hole circle (at -5.625 -1.27 0) (size 1.6 1.6) (drill 0.9) (layers *.Cu *.Mask) (net {{net_B_id}} "{{net_B}}"))
          (pad C thru_hole circle (at -5.625 1.27 0) (size 1.6 1.6) (drill 0.9) (layers *.Cu *.Mask) (net {{net_C_id}} "{{net_C}}"))
          (pad D thru_hole circle (at -5.625 3.81 0) (size 1.6 1.6) (drill 0.9) (layers *.Cu *.Mask) (net {{net_D_id}} "{{net_D}}"))

          
          (pad "" np_thru_hole circle (at -5.625 6.3 0) (size 1.5 1.5) (drill 1.5) (layers *.Cu *.Mask))
        )