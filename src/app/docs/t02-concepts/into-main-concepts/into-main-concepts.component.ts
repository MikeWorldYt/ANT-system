import { Component } from '@angular/core';
import { FooterComponent } from '../../../layout/footer/footer.component';
import { Docs_T02_Mod01_Component } from '../mod-01-levels-structure/mod-01-levels-structure.component';

@Component({
  selector: 'docs-t02-into-main-concepts',
  standalone: true,
  imports: [
    FooterComponent,
    Docs_T02_Mod01_Component,
  ],
  templateUrl: './into-main-concepts.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_IntoComponent {

}
