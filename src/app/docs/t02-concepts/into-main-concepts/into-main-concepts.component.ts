import { Component, OnInit } from '@angular/core';
import { FooterComponent } from '../../../layout/footer/footer.component';
import { TitleService } from '../../../services/navTitle.service';
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
    private TitleService: TitleService
  ) {}

  ngOnInit(): void {
    // Set TitleValue Service
    this.TitleService.setTitle('concepts');
  }

  toggle(section: string) {
    this.currTitle = section;
  }

}
