import { Component, OnInit } from '@angular/core';
import { FooterComponent } from '../../../layout/footer/footer.component';
import { TitleStateService } from '../../../services/title.service';
import { RouterOutlet } from '@angular/router';

@Component({
  selector: 'docs-t02-into-main-concepts',
  standalone: true,
  imports: [
    FooterComponent,
    RouterOutlet
  ],
  templateUrl: './into-main-concepts.component.html',
  styleUrl: '../../docs.component.css'
})
export class Docs_T02_IntoComponent implements OnInit {
  currTitle: string = 'introduction';

  constructor(
    private titleStateService: TitleStateService
  ) {}

  ngOnInit(): void {
    // Set title section
    this.titleStateService.setCurrTitle('concepts');
  }

  toggle(section: string) {
    this.currTitle = section;
  }

}
